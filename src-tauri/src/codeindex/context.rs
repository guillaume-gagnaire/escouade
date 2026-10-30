//! Ported from tokensave `src/context/builder.rs` (`ContextBuilder::build_context` and its
//! helpers), the defaults of `BuildContextOptions` (`src/types.rs`), the options of the
//! `tokensave_context` handler (`src/mcp/tools/handlers/graph.rs`) and the breadth-first walk of
//! `GraphTraverser::traverse_bfs` (`src/graph/traversal.rs`) (github.com/aovestdipaperino/tokensave,
//! commit 580936b, MIT, © 2025 Enzo Lombardi; see THIRD-PARTY-NOTICES.md): the context of a task.
//!
//! The pipeline, the same steps and constants: the words of the task (identifiers, compounds of
//! neighbouring words, stop words), the exact source (`Type::Variant`, a quoted phrase), a bounded
//! top-K per term merged round-robin, the exact names at 100, the bodies of the functions, the
//! methods of the types found at 0.5×, the multiplicative re-ranking, the callers, the
//! co-occurrence, the cap per file, three roots of graph at depth 1, 20 nodes at most.
//!
//! What differs, for our architecture: the counts of branches and loops of a body are read from its
//! words (our index keeps no complexity); a truncated snippet points at `index_body`, not
//! `tokensave_body`; the ids are numbers; every query sees only what the folder asking sees (the
//! worktree's delta) and reads no file the fence refuses. Not ported: `merge_adjacent` and the
//! `plan` mode of the handler.

use super::api::{Hit, Node, Reader, Retrieval, Scored, Snippet, TermHits, NODE_COLS};
use super::filters::PathFilter;
use super::ranking::{
    apply_connectivity_boost, apply_executable_intent_boost, is_api_owner_kind, is_executable_kind,
    rerank_candidates,
};
use super::text::{is_camel_case, split_compound};
use anyhow::Result;
use std::collections::{HashMap, HashSet, VecDeque};

/// Options for building the context of a task (tokensave `BuildContextOptions`, its defaults).
#[derive(Debug, Clone)]
pub struct BuildContextOptions {
    pub max_nodes: usize,
    pub max_code_blocks: usize,
    pub max_code_block_size: usize,
    pub include_code: bool,
    pub search_limit: usize,
    pub traversal_depth: usize,
    pub min_score: f64,
    /// Additional keywords to search for beyond those extracted from the query.
    pub extra_keywords: Vec<String>,
    /// Maximum symbols from a single file. `None`: `max_nodes`.
    pub max_per_file: Option<usize>,
    pub max_code_lines: Option<usize>,
    /// Node ids to exclude from results (for session deduplication across calls).
    pub exclude_node_ids: HashSet<i64>,
    /// `path_prefix`, `path_include`, `path_exclude` and the project's `queryignore`: the entry
    /// points kept (graph expansion may still traverse outside them).
    pub filter: PathFilter,
}

impl Default for BuildContextOptions {
    fn default() -> Self {
        BuildContextOptions {
            max_nodes: 20,
            max_code_blocks: 5,
            max_code_block_size: 1500,
            include_code: true,
            search_limit: 3,
            traversal_depth: 1,
            min_score: 0.0,
            extra_keywords: Vec::new(),
            max_per_file: None,
            max_code_lines: None,
            exclude_node_ids: HashSet::new(),
            filter: PathFilter::default(),
        }
    }
}

impl BuildContextOptions {
    /// The options the `tokensave_context` handler builds: `max_nodes` (20 by default, 100 at
    /// most), the cap per file at a third of it (3 at least), the keywords.
    pub fn of_tool(max_nodes: Option<usize>, keywords: Vec<String>) -> Self {
        Self::of_args(ToolArgs {
            max_nodes,
            keywords,
            ..Default::default()
        })
    }

    /// The options of the handler from all its arguments (tokensave `handle_context`):
    /// `max_code_blocks` 5 by default, 20 at most; `max_code_lines` between 1 and 1000;
    /// `max_per_file` a third of `max_nodes`, 3 at least.
    pub fn of_args(a: ToolArgs) -> Self {
        let max_nodes = a.max_nodes.map_or(20, |v| v.min(100)).max(1);
        BuildContextOptions {
            max_nodes,
            include_code: a.include_code.unwrap_or(true),
            max_code_blocks: a.max_code_blocks.map_or(5, |v| v.min(20)).max(1),
            max_code_lines: a.max_code_lines.map(|v| v.clamp(1, 1000)),
            extra_keywords: a.keywords,
            exclude_node_ids: a.exclude_node_ids.into_iter().collect(),
            max_per_file: Some(a.max_per_file.unwrap_or((max_nodes / 3).max(3))),
            filter: a.filter,
            ..Default::default()
        }
    }
}

/// The arguments of `index_context` that make its options.
#[derive(Debug, Clone, Default)]
pub struct ToolArgs {
    pub max_nodes: Option<usize>,
    pub include_code: Option<bool>,
    pub max_code_blocks: Option<usize>,
    pub max_code_lines: Option<usize>,
    pub keywords: Vec<String>,
    pub exclude_node_ids: Vec<i64>,
    pub max_per_file: Option<usize>,
    pub filter: PathFilter,
}

/// What `build_context` gives.
pub struct Built {
    pub entry_points: Vec<Node>,
    pub subgraph: Vec<Node>,
    pub code_blocks: Vec<Snippet>,
    pub diagnostics: Retrieval,
}

impl Reader {
    /// Builds the context of a task (tokensave `build_context`).
    pub(super) fn build_context(
        &self,
        query: &str,
        options: &BuildContextOptions,
    ) -> Result<Built> {
        let symbols = extract_symbols_from_query(query);
        let (entry_points, diagnostics) = self.find_entry_points(query, &symbols, options)?;
        let subgraph = self.expand_subgraph(&entry_points, options)?;
        let code_blocks = if options.include_code {
            self.extract_code_blocks(&entry_points, options)
        } else {
            Vec::new()
        };
        Ok(Built {
            entry_points,
            subgraph,
            code_blocks,
            diagnostics,
        })
    }

    fn find_entry_points(
        &self,
        query: &str,
        symbols: &[String],
        options: &BuildContextOptions,
    ) -> Result<(Vec<Node>, Retrieval)> {
        // Base score for an exact name match.
        const EXACT_MATCH_SCORE: f64 = 100.0;
        // Floor for the "strong" diagnostics tier.
        const STRONG_MATCH_SCORE: f64 = 10.0;
        let mut index_of: HashMap<i64, usize> = HashMap::new();
        let mut candidates: Vec<Scored> = Vec::new();
        let mut literal_terms = exact_source_terms(query, &options.extra_keywords);
        literal_terms.extend(exact_phrase_terms(query, &options.extra_keywords));
        let excluded = &options.exclude_node_ids;
        let exact_source_candidates = self.find_exact_source_candidates(&literal_terms, options)?;
        let exact_source_ids: HashSet<i64> = exact_source_candidates
            .iter()
            .map(|candidate| candidate.node.id)
            .collect();
        for candidate in exact_source_candidates {
            index_of.insert(candidate.node.id, candidates.len());
            candidates.push(candidate);
        }
        let cap = options.max_nodes * 2;

        let mut fts_terms: Vec<String> = Vec::new();
        let mut fts_seen: HashSet<String> = HashSet::new();
        let push_term = |t: String, terms: &mut Vec<String>, seen: &mut HashSet<String>| {
            if !t.is_empty() && seen.insert(t.clone()) {
                terms.push(t);
            }
        };
        let body_terms = conceptual_query_terms(query, &options.extra_keywords);
        for term in &body_terms {
            push_term(term.clone(), &mut fts_terms, &mut fts_seen);
        }
        for s in symbols {
            push_term(s.clone(), &mut fts_terms, &mut fts_seen);
        }
        for k in &options.extra_keywords {
            push_term(k.clone(), &mut fts_terms, &mut fts_seen);
        }

        // Each term's ranked top-K, then the pool filled round-robin.
        let fetch_limit = (options.search_limit * 3).max(10);
        let mut per_term: Vec<VecDeque<Scored>> = Vec::with_capacity(fts_terms.len());
        for term in &fts_terms {
            per_term.push(self.search_nodes_bounded(term, fetch_limit)?.into());
        }
        let term_hits: Vec<TermHits> = fts_terms
            .iter()
            .zip(&per_term)
            .map(|(term, queue)| TermHits {
                term: term.clone(),
                hits: queue.len(),
            })
            .collect();
        'fill: loop {
            let mut advanced = false;
            for queue in &mut per_term {
                if candidates.len() >= cap {
                    break 'fill;
                }
                let Some(sr) = queue.pop_front() else {
                    continue;
                };
                advanced = true;
                if !score_passes(sr.score, options.min_score) || excluded.contains(&sr.node.id) {
                    continue;
                }
                if let Some(&idx) = index_of.get(&sr.node.id) {
                    candidates[idx].score = candidates[idx].score.max(sr.score);
                } else {
                    index_of.insert(sr.node.id, candidates.len());
                    candidates.push(sr);
                }
            }
            if !advanced {
                break;
            }
        }

        // --- Exact name supplement: only the symbols the user wrote ---
        let mut exact_names: Vec<String> = symbols
            .iter()
            .filter(|s| !s.contains("::") && s.len() >= 3)
            .filter(|s| is_authored_symbol(s, query, &options.extra_keywords))
            .cloned()
            .collect();
        if query.split_once(' ').is_some_and(|(verb, path)| {
            matches!(
                verb,
                "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS"
            ) && path.starts_with('/')
        }) {
            exact_names.push(query.to_string());
        }
        let mut exact_ids: HashSet<i64> = exact_source_ids.clone();
        if !exact_names.is_empty() {
            for node in self.search_nodes_by_exact_name(&exact_names, options.search_limit, None)? {
                if excluded.contains(&node.id) {
                    continue;
                }
                exact_ids.insert(node.id);
                if let Some(&idx) = index_of.get(&node.id) {
                    candidates[idx].score = candidates[idx].score.max(EXACT_MATCH_SCORE);
                } else {
                    index_of.insert(node.id, candidates.len());
                    candidates.push(Scored {
                        node,
                        score: EXACT_MATCH_SCORE,
                    });
                }
            }
        }

        // --- Executable-body supplement ---
        if body_terms.len() >= 2 {
            for sr in self.find_executable_body_candidates(&body_terms, options)? {
                if excluded.contains(&sr.node.id) {
                    continue;
                }
                if let Some(&idx) = index_of.get(&sr.node.id) {
                    candidates[idx].score = candidates[idx].score.max(sr.score);
                } else {
                    index_of.insert(sr.node.id, candidates.len());
                    candidates.push(sr);
                }
            }
        }

        // --- Requested API-family supplement: the methods of the types found, at half ---
        let owner_scores: HashMap<i64, f64> = candidates
            .iter()
            .filter(|candidate| is_api_owner_kind(&candidate.node.kind))
            .map(|owner| (owner.node.id, owner.score))
            .collect();
        let owners: Vec<Node> = candidates
            .iter()
            .filter(|c| owner_scores.contains_key(&c.node.id))
            .map(|c| c.node.clone())
            .collect();
        for (owner, child) in self.get_children_of_many(&owners)? {
            if !is_executable_kind(&child.kind) || excluded.contains(&child.id) {
                continue;
            }
            let score = owner_scores.get(&owner).copied().unwrap_or_default() * 0.5;
            if let Some(&idx) = index_of.get(&child.id) {
                candidates[idx].score = candidates[idx].score.max(score);
            } else {
                index_of.insert(child.id, candidates.len());
                candidates.push(Scored { node: child, score });
            }
        }

        // --- path_prefix, path_include / path_exclude and queryignore filters ---
        if !options.filter.is_empty() {
            candidates.retain(|sr| options.filter.keeps(&sr.node.path));
        }

        // --- Re-rank with structural signals ---
        rerank_candidates(&mut candidates);
        apply_executable_intent_boost(&mut candidates, query);

        // --- Connectivity boost ---
        let nodes: Vec<&Node> = candidates.iter().map(|c| &c.node).collect();
        if let Ok(call_counts) = self.batch_incoming_call_counts(&nodes) {
            apply_connectivity_boost(&mut candidates, &call_counts);
        }

        // --- Co-occurrence boost for multi-term queries ---
        let query_terms: Vec<String> = query
            .split_whitespace()
            .map(str::to_lowercase)
            .filter(|w| w.len() >= 3)
            .collect();
        if query_terms.len() >= 2 {
            apply_cooccurrence_boost(&mut candidates, &query_terms);
        }

        // --- Diagnostics ---
        let best: Option<(i64, f64)> = candidates
            .iter()
            .map(|candidate| (candidate.node.id, candidate.score))
            .fold(None, |acc: Option<(i64, f64)>, (id, score)| match acc {
                Some((_, top)) if top >= score => acc,
                _ => Some((id, score)),
            });
        let best_score = best.map(|(_, score)| score);
        let match_quality = best.map(|(id, score)| {
            if exact_ids.contains(&id) {
                "exact".to_string()
            } else if score >= STRONG_MATCH_SCORE {
                "strong".to_string()
            } else {
                "fts-only".to_string()
            }
        });
        let diagnostics = Retrieval {
            terms: term_hits,
            best_score,
            quality: match_quality,
        };

        // --- Per-file diversity cap + final BFS root limit ---
        let max_per_file = options.max_per_file.unwrap_or(options.max_nodes);
        let (mut exact_candidates, semantic_candidates): (Vec<_>, Vec<_>) = candidates
            .into_iter()
            .partition(|candidate| exact_source_ids.contains(&candidate.node.id));
        exact_candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut entry_points: Vec<Node> = exact_candidates
            .into_iter()
            .take(options.max_nodes)
            .map(|candidate| candidate.node)
            .collect();
        let semantic_slots = options
            .search_limit
            .min(options.max_nodes.saturating_sub(entry_points.len()));
        if semantic_slots > 0 {
            entry_points.extend(apply_per_file_cap(
                semantic_candidates,
                semantic_slots,
                max_per_file,
            ));
        }
        Ok((entry_points, diagnostics))
    }

    /// The files this folder sees, the removed ones left out.
    pub(super) fn visible_files(&self) -> Result<Vec<String>> {
        let sql = "SELECT DISTINCT f.path FROM files f WHERE f.state <> 2
               AND (f.scope = ?1 OR (f.scope = '' AND NOT EXISTS (SELECT 1 FROM files fw
                   WHERE fw.scope = ?1 AND fw.path = f.path AND fw.state IN (1, 2))))
             ORDER BY f.path";
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([&self.scope], |r| r.get::<_, String>(0))?;
        Ok(rows.flatten().collect())
    }

    pub(super) fn nodes_of_file(&self, path: &str) -> Result<Vec<Node>> {
        let sql = format!(
            "SELECT {NODE_COLS} FROM symbols s WHERE s.path = ?1 AND {}",
            Self::vis_pos("s", 2)
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params![path, self.scope], Node::of)?;
        Ok(rows.flatten().collect())
    }

    /// The innermost symbol enclosing each exact source expression (tokensave
    /// `find_exact_source_candidates`).
    fn find_exact_source_candidates(
        &self,
        terms: &[String],
        options: &BuildContextOptions,
    ) -> Result<Vec<Scored>> {
        const EXACT_SOURCE_SCORE: f64 = 1_000.0;
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let mut candidates: HashMap<i64, Scored> = HashMap::new();
        for path in self.visible_files()? {
            if !options.filter.keeps(&path) {
                continue;
            }
            let Some(source) = self.read_file(&path) else {
                continue;
            };
            if !terms.iter().any(|term| source.contains(term)) {
                continue;
            }
            let nodes = self.nodes_of_file(&path)?;
            for (line_index, line) in source.lines().enumerate() {
                let hit_count = terms.iter().filter(|term| line.contains(*term)).count();
                if hit_count == 0 {
                    continue;
                }
                let line1 = line_index as u32 + 1;
                let enclosing = nodes
                    .iter()
                    .filter(|node| {
                        node.line <= line1
                            && line1 <= node.end_line
                            && !options.exclude_node_ids.contains(&node.id)
                    })
                    .min_by_key(|node| node.end_line.saturating_sub(node.line));
                let Some(node) = enclosing else {
                    continue;
                };
                candidates
                    .entry(node.id)
                    .and_modify(|candidate| candidate.score += hit_count as f64)
                    .or_insert_with(|| Scored {
                        node: node.clone(),
                        score: EXACT_SOURCE_SCORE + hit_count as f64,
                    });
            }
        }
        let mut candidates: Vec<Scored> = candidates.into_values().collect();
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.node.path.cmp(&b.node.path))
                .then_with(|| a.node.line.cmp(&b.node.line))
        });
        Ok(candidates)
    }

    /// Executable symbols whose bodies hold several conceptual terms (tokensave
    /// `find_executable_body_candidates`).
    fn find_executable_body_candidates(
        &self,
        terms: &[String],
        options: &BuildContextOptions,
    ) -> Result<Vec<Scored>> {
        let mut results = self
            .search_executable_bodies(terms, options.max_nodes.saturating_mul(4))?
            .into_iter()
            .filter(|(node, _, _)| options.filter.keeps(&node.path))
            .map(|(node, hits, branches_loops)| {
                let control_flow_boost = f64::from(branches_loops).min(4.0) * 0.25;
                Scored {
                    node,
                    score: 1.5 + hits as f64 + control_flow_boost,
                }
            })
            .collect::<Vec<_>>();
        results.truncate(options.max_nodes * 2);
        Ok(results)
    }

    /// The children of each owner: the symbols it holds (its line range, its name as their
    /// holder), in file and line order, with the owner's id (tokensave `get_children_of_many`).
    fn get_children_of_many(&self, owners: &[Node]) -> Result<Vec<(i64, Node)>> {
        let mut out = Vec::new();
        let sql = format!(
            "SELECT {NODE_COLS} FROM symbols s
             WHERE s.path = ?1 AND s.parent = ?2 AND s.line >= ?3 AND s.end_line <= ?4 AND s.id <> ?5
               AND {} ORDER BY s.path, s.line",
            Self::vis_pos("s", 6)
        );
        let mut stmt = self.conn.prepare(&sql)?;
        for o in owners {
            let holder = if o.parent.is_empty() {
                o.name.clone()
            } else {
                format!("{}.{}", o.parent, o.name)
            };
            let rows = stmt.query_map(
                rusqlite::params![o.path, holder, o.line, o.end_line, o.id, self.scope],
                Node::of,
            )?;
            out.extend(rows.flatten().map(|n| (o.id, n)));
        }
        out.sort_by(|a, b| (&a.1.path, a.1.line).cmp(&(&b.1.path, b.1.line)));
        Ok(out)
    }

    /// The neighbours of a node at one step, `calls` first, outgoing then incoming (tokensave
    /// `traverse_bfs` with `TraversalDirection::Both` and `edge_kind_priority`), by the resolved
    /// links (ESC-297).
    fn neighbours(&self, node: &Node) -> Result<Vec<Node>> {
        let linked = |sql: String| -> Result<Vec<Node>> {
            let mut stmt = self.conn.prepare(&sql)?;
            let ids: Vec<i64> = stmt
                .query_map(rusqlite::params![node.id, self.scope], |r| r.get(0))?
                .flatten()
                .collect();
            let by_id: HashMap<i64, Node> = self
                .nodes_by_ids(&ids)?
                .into_iter()
                .map(|n| (n.id, n))
                .collect();
            let mut seen = HashSet::new();
            Ok(ids
                .into_iter()
                .filter(|i| seen.insert(*i))
                .filter_map(|i| by_id.get(&i).cloned())
                .collect())
        };
        let targets = |kinds: &str| {
            linked(format!(
                "SELECT l.to_id FROM links l WHERE l.from_id = ?1 AND l.kind IN {kinds} AND {}
                 ORDER BY l.line",
                Self::lvis_pos("l", 2)
            ))
        };
        let holders = |kinds: &str| {
            linked(format!(
                "SELECT l.from_id FROM links l WHERE l.to_id = ?1 AND l.kind IN {kinds}
                 AND l.from_id IS NOT NULL AND {} ORDER BY l.path, l.line",
                Self::lvis_pos("l", 2)
            ))
        };
        let mut out: Vec<Node> = Vec::new();
        out.extend(targets("('call')")?);
        out.extend(holders("('call')")?);
        out.extend(targets("('extends','implements')")?);
        out.extend(holders("('extends','implements')")?);
        Ok(out)
    }

    /// The subgraph around the entry points (tokensave `expand_subgraph`): each root walked at
    /// `traversal_depth` (1), the whole held to `max_nodes`.
    fn expand_subgraph(
        &self,
        entry_points: &[Node],
        options: &BuildContextOptions,
    ) -> Result<Vec<Node>> {
        let mut all_nodes: Vec<Node> = Vec::new();
        let mut seen_node_ids: HashSet<i64> = HashSet::new();
        for node in entry_points {
            let mut visited: HashSet<i64> = HashSet::from([node.id]);
            let mut result_nodes: Vec<Node> = vec![node.clone()];
            let mut queue: VecDeque<(Node, usize)> = VecDeque::from([(node.clone(), 0)]);
            while let Some((current, depth)) = queue.pop_front() {
                if depth >= options.traversal_depth || result_nodes.len() >= options.max_nodes {
                    break;
                }
                for n in self.neighbours(&current)? {
                    if !visited.insert(n.id) {
                        continue;
                    }
                    result_nodes.push(n.clone());
                    queue.push_back((n, depth + 1));
                    if result_nodes.len() >= options.max_nodes {
                        break;
                    }
                }
            }
            for n in result_nodes {
                if seen_node_ids.insert(n.id) {
                    all_nodes.push(n);
                }
            }
            if all_nodes.len() >= options.max_nodes {
                break;
            }
        }
        all_nodes.truncate(options.max_nodes);
        Ok(all_nodes)
    }

    /// The code of the entry points (tokensave `extract_code_blocks`).
    fn extract_code_blocks(
        &self,
        entry_points: &[Node],
        options: &BuildContextOptions,
    ) -> Vec<Snippet> {
        let mut blocks = Vec::new();
        for node in entry_points {
            if blocks.len() >= options.max_code_blocks {
                break;
            }
            if let Some(code) = self.lines(&node.path, node.line, node.end_line) {
                if code.is_empty() {
                    continue;
                }
                blocks.push(Snippet {
                    id: node.id,
                    path: node.path.clone(),
                    line: node.line,
                    end_line: node.end_line,
                    text: truncate_code_block(
                        &code,
                        options.max_code_block_size,
                        options.max_code_lines,
                        node,
                    ),
                });
            }
        }
        blocks
    }
}

impl Node {
    pub(super) fn to_hit(&self) -> Hit {
        Hit {
            id: self.id,
            name: self.name.clone(),
            kind: self.kind.clone(),
            parent: self.parent.clone(),
            path: self.path.clone(),
            line: self.line,
            end_line: self.end_line,
            sig: self.sig.clone(),
            doc: self.doc.clone(),
        }
    }
}

/// Whether a search score passes the minimum threshold.
fn score_passes(score: f64, min_score: f64) -> bool {
    score > 0.0 && score >= min_score
}

/// Extracts potential symbol names from natural language text: CamelCase, snake_case,
/// SCREAMING_SNAKE, `a::b` paths, and the Pascal/camel compounds of two neighbouring plain words.
pub fn extract_symbols_from_query(query: &str) -> Vec<String> {
    let stop_words: HashSet<&str> = SYMBOL_STOP_WORDS.iter().copied().collect();
    let mut symbols: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut plain_words: Vec<Option<String>> = Vec::new();
    for token in query.split_whitespace() {
        let clean = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != ':');
        classify_token(clean, &stop_words, &mut symbols, &mut seen);
        let is_plain = clean.len() >= 3
            && clean.chars().all(|c| c.is_ascii_lowercase())
            && !stop_words.contains(clean.to_lowercase().as_str());
        plain_words.push(is_plain.then(|| clean.to_string()));
    }
    for pair in plain_words.windows(2) {
        if let [Some(a), Some(b)] = pair {
            let cap = |w: &str| {
                let mut c = w.chars();
                c.next().map_or_else(String::new, |f| {
                    f.to_ascii_uppercase().to_string() + c.as_str()
                })
            };
            let pascal = format!("{}{}", cap(a), cap(b));
            if seen.insert(pascal.clone()) {
                symbols.push(pascal);
            }
            let camel = format!("{a}{}", cap(b));
            if seen.insert(camel.clone()) {
                symbols.push(camel);
            }
        }
    }
    symbols
}

/// Stop words filtered out during symbol extraction from natural language.
const SYMBOL_STOP_WORDS: &[&str] = &[
    "the",
    "is",
    "in",
    "for",
    "to",
    "a",
    "an",
    "of",
    "and",
    "or",
    "not",
    "this",
    "that",
    "it",
    "with",
    "on",
    "at",
    "by",
    "from",
    "as",
    "be",
    "was",
    "are",
    "been",
    "being",
    "have",
    "has",
    "had",
    "do",
    "does",
    "did",
    "will",
    "would",
    "could",
    "should",
    "may",
    "might",
    "can",
    "shall",
    "how",
    "what",
    "where",
    "when",
    "who",
    "which",
    "why",
    "if",
    "then",
    "else",
    "but",
    "so",
    "up",
    "out",
    "no",
    "yes",
    "all",
    "any",
    "each",
    "every",
    "fix",
    "look",
    "update",
    "add",
    "remove",
    "delete",
    "change",
    "check",
    "find",
    "get",
    "set",
    "use",
    "make",
    "call",
    "function",
    "method",
    "singleton_method",
    "class",
    "struct",
    "type",
    "module",
    "file",
    "handler",
    "implement",
    "create",
    "about",
    // Code-specific noise words (ported from codegraph)
    "interface",
    "trait",
    "enum",
    "variable",
    "import",
    "export",
    "return",
    "error",
    "test",
    "spec",
    "helper",
    "util",
    "config",
    "service",
    "model",
    "view",
    "controller",
    "code",
    "new",
    "init",
    "default",
    "value",
    "data",
    "result",
];

/// Classify a single cleaned token and push any symbols it yields.
fn classify_token(
    clean: &str,
    stop_words: &HashSet<&str>,
    symbols: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    if clean.is_empty() {
        return;
    }
    if clean.contains("::") {
        if let Some(last) = clean.rsplit("::").next() {
            if !last.is_empty()
                && !stop_words.contains(last.to_lowercase().as_str())
                && seen.insert(last.to_string())
            {
                symbols.push(last.to_string());
            }
        }
        let full = clean.to_string();
        if seen.insert(full.clone()) {
            symbols.push(full);
        }
        return;
    }
    if clean.contains('_') {
        if !stop_words.contains(clean.to_lowercase().as_str()) && seen.insert(clean.to_string()) {
            symbols.push(clean.to_string());
        }
        for part in split_compound(clean) {
            if part.len() >= 3
                && !stop_words.contains(part.to_lowercase().as_str())
                && seen.insert(part.to_string())
            {
                symbols.push(part.to_string());
            }
        }
        return;
    }
    if is_camel_case(clean) {
        if !stop_words.contains(clean.to_lowercase().as_str()) && seen.insert(clean.to_string()) {
            symbols.push(clean.to_string());
        }
        for part in split_compound(clean) {
            if part.len() >= 3
                && !stop_words.contains(part.to_lowercase().as_str())
                && seen.insert(part.to_string())
            {
                symbols.push(part.to_string());
            }
        }
    }
}

/// Whether `symbol` came from the user's own words: a compound, a verbatim token, a keyword.
fn is_authored_symbol(symbol: &str, query: &str, extra_keywords: &[String]) -> bool {
    split_compound(symbol).len() >= 2
        || query
            .split_whitespace()
            .any(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '_') == symbol)
        || extra_keywords.iter().any(|k| k == symbol)
}

/// Shortest phrase accepted as exact-copy evidence.
const MIN_EXACT_PHRASE_LEN: usize = 8;

/// Cap on phrase terms, so a keyword list can't make the source scan unbounded.
const MAX_EXACT_PHRASES: usize = 8;

/// The multi-word phrases matched verbatim in the source: keywords with inner whitespace, and the
/// double-quoted spans of the task.
fn exact_phrase_terms(query: &str, extra_keywords: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut phrases = Vec::new();
    let mut push = |candidate: &str, phrases: &mut Vec<String>| {
        let phrase = candidate.trim();
        if phrase.len() >= MIN_EXACT_PHRASE_LEN
            && phrase.split_whitespace().count() >= 2
            && seen.insert(phrase.to_string())
        {
            phrases.push(phrase.to_string());
        }
    };
    for keyword in extra_keywords {
        push(keyword, &mut phrases);
    }
    let mut rest = query;
    while let Some(open) = rest.find('"') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('"') else { break };
        push(&after[..close], &mut phrases);
        rest = &after[close + 1..];
    }
    phrases.truncate(MAX_EXACT_PHRASES);
    phrases
}

/// The exact, namespace-qualified source tokens (`Type::Variant`) of the task and keywords.
fn exact_source_terms(query: &str, extra_keywords: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    std::iter::once(query)
        .chain(extra_keywords.iter().map(String::as_str))
        .flat_map(str::split_whitespace)
        .filter_map(|word| {
            let token = word
                .trim_matches(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':' || c == '#'));
            let mut segments = token.split("::");
            let first = segments.next()?;
            let rest: Vec<&str> = segments.collect();
            let valid_segment = |segment: &str| {
                !segment.is_empty()
                    && segment
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_' || c == '#')
            };
            (!rest.is_empty()
                && valid_segment(first)
                && rest.iter().all(|segment| valid_segment(segment))
                && seen.insert(token.to_string()))
            .then(|| token.to_string())
        })
        .collect()
}

/// Lower-case concept words for the term searches and the body co-occurrence search.
fn conceptual_query_terms(query: &str, extra_keywords: &[String]) -> Vec<String> {
    const STOP: &[&str] = &[
        "about", "after", "before", "code", "find", "from", "function", "harden", "into", "locate",
        "near", "request", "that", "their", "then", "this", "with",
    ];
    let mut seen = HashSet::new();
    query
        .split_whitespace()
        .chain(extra_keywords.iter().flat_map(|s| s.split_whitespace()))
        .filter_map(|word| {
            let normalized: String = word
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .flat_map(char::to_lowercase)
                .collect();
            (normalized.len() >= 4
                && !STOP.contains(&normalized.as_str())
                && seen.insert(normalized.clone()))
            .then_some(normalized)
        })
        .collect()
}

/// Boosts candidates whose name, qualified name or path holds two query terms or more.
fn apply_cooccurrence_boost(candidates: &mut [Scored], query_terms: &[String]) {
    for candidate in candidates.iter_mut() {
        let haystack = format!(
            "{} {} {}",
            candidate.node.name.to_lowercase(),
            candidate.node.qualified().to_lowercase(),
            candidate.node.path.to_lowercase(),
        );
        let hits: usize = query_terms
            .iter()
            .filter(|term| haystack.contains(term.as_str()))
            .count();
        if hits >= 2 {
            // Boost proportional to coverage: 2 terms → 1.3×, 3 → 1.6×, etc.
            candidate.score *= 1.0 + (hits as f64 - 1.0) * 0.3;
        }
    }
    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// The top `max_total` candidates, at most `max_per_file` from one file, a slot kept for a
/// file's executable owner; the rest after, if there is room.
fn apply_per_file_cap(candidates: Vec<Scored>, max_total: usize, max_per_file: usize) -> Vec<Node> {
    let files_with_owner: HashSet<String> = candidates
        .iter()
        .filter(|candidate| is_executable_kind(&candidate.node.kind))
        .map(|candidate| candidate.node.path.clone())
        .collect();
    let mut file_counts: HashMap<String, usize> = HashMap::new();
    let mut owner_accepted: HashSet<String> = HashSet::new();
    let mut accepted: Vec<Node> = Vec::new();
    let mut spillover: Vec<Node> = Vec::new();
    for sr in candidates {
        let file_path = sr.node.path.clone();
        let is_owner = is_executable_kind(&sr.node.kind);
        let count = file_counts.entry(file_path.clone()).or_insert(0);
        let reserve_owner_slot = !is_owner
            && files_with_owner.contains(&file_path)
            && !owner_accepted.contains(&file_path)
            && count.saturating_add(1) >= max_per_file;
        if *count < max_per_file && !reserve_owner_slot {
            *count += 1;
            if is_owner {
                owner_accepted.insert(file_path);
            }
            accepted.push(sr.node);
        } else {
            spillover.push(sr.node);
        }
        if accepted.len() >= max_total {
            break;
        }
    }
    for node in spillover {
        if accepted.len() >= max_total {
            break;
        }
        accepted.push(node);
    }
    accepted
}

/// Truncates a snippet to `max_lines` lines and `max_size` bytes, whichever bites first,
/// preferring a line boundary; a truncated snippet ends by the lines that follow.
fn truncate_code_block(
    code: &str,
    max_size: usize,
    max_lines: Option<usize>,
    node: &Node,
) -> String {
    let capped = match max_lines {
        Some(limit) => line_prefix(code, limit),
        None => code,
    };
    if capped.len() <= max_size {
        return if capped.len() == code.len() {
            code.to_string()
        } else {
            with_body_handle(capped, node)
        };
    }
    let mut end = max_size.min(capped.len());
    while !capped.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    let prefix = &capped[..end];
    let end = prefix.rfind('\n').unwrap_or(prefix.len());
    with_body_handle(&prefix[..end], node)
}

/// The prefix of `code` spanning at most `max_lines` lines (0: no cap).
fn line_prefix(code: &str, max_lines: usize) -> &str {
    if max_lines == 0 {
        return code;
    }
    match code.match_indices('\n').nth(max_lines - 1) {
        Some((idx, _)) if idx + 1 == code.len() => code,
        Some((idx, _)) => &code[..idx],
        None => code,
    }
}

/// Appends the follow-up handle that points at the full body.
fn with_body_handle(body: &str, node: &Node) -> String {
    tr!(
        "{body}\n... [tronqué — le corps entier : index_body node_id={id}]",
        "{body}\n... [truncated — full body: index_body node_id={id}]",
        body = body,
        id = node.id
    )
}

/// The branches and loops of a body, read from its words: tokensave counts them from the tree
/// (`branches + loops`), our index keeps no complexity.
pub(super) fn branches_and_loops(body: &str) -> u32 {
    const CONTROL: &[&str] = &[
        "if", "elif", "match", "switch", "case", "for", "foreach", "while", "loop", "catch",
        "except",
    ];
    body.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| CONTROL.contains(w))
        .count() as u32
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn test_spaced_words_generate_camel_bigrams() {
        let symbols = extract_symbols_from_query("redesign the promo banner layout");
        assert!(symbols.contains(&"PromoBanner".to_string()), "{symbols:?}");
        assert!(symbols.contains(&"promoBanner".to_string()), "{symbols:?}");
    }

    #[test]
    fn test_extract_snake_case() {
        let symbols = extract_symbols_from_query("fix the process_request function");
        assert!(symbols.contains(&"process_request".to_string()));
    }

    #[test]
    fn test_authored_symbol_multiword_compound_passes() {
        assert!(is_authored_symbol("PromoBanner", "anything", &[]));
        assert!(is_authored_symbol("process_request", "anything", &[]));
    }

    #[test]
    fn test_authored_symbol_verbatim_token_passes() {
        let q = "how does tokensave_context build its result?";
        assert!(is_authored_symbol("tokensave_context", q, &[]));
    }

    #[test]
    fn test_authored_symbol_trims_punctuation() {
        assert!(is_authored_symbol(
            "tokensave_context",
            "call tokensave_context.",
            &[]
        ));
    }

    #[test]
    fn test_authored_symbol_derived_segment_rejected() {
        let q = "how does tokensave_context build its result?";
        assert!(!is_authored_symbol("tokensave", q, &[]));
    }

    #[test]
    fn test_authored_symbol_extra_keyword_passes() {
        assert!(is_authored_symbol(
            "ranker",
            "unrelated query",
            &["ranker".to_string()]
        ));
    }

    #[test]
    fn test_extract_camel_case() {
        let symbols = extract_symbols_from_query("update UserService handler");
        assert!(symbols.contains(&"UserService".to_string()));
    }

    #[test]
    fn test_extract_screaming_snake() {
        let symbols = extract_symbols_from_query("increase MAX_RETRIES limit");
        assert!(symbols.contains(&"MAX_RETRIES".to_string()));
    }

    #[test]
    fn test_extract_qualified_path() {
        let symbols = extract_symbols_from_query("look at crate::types::Node");
        assert!(symbols.iter().any(|s| s.contains("Node")));
    }

    #[test]
    fn test_exact_source_terms_preserve_qualified_expressions() {
        let terms = exact_source_terms(
            "Find every path for `BasisOrder::Linear`, not generic BasisOrder.",
            &[
                "BasisOrder::Linear".to_string(),
                "crate::types::Node".to_string(),
            ],
        );
        assert_eq!(
            terms,
            vec![
                "BasisOrder::Linear".to_string(),
                "crate::types::Node".to_string()
            ]
        );
    }

    #[test]
    fn exact_phrase_terms_takes_multiword_keywords_verbatim() {
        let terms = exact_phrase_terms(
            "Diagnose the dashboard",
            &["Waiting for status".to_string()],
        );
        assert_eq!(terms, vec!["Waiting for status".to_string()]);
    }

    #[test]
    fn exact_phrase_terms_rejects_single_words_and_short_phrases() {
        let terms = exact_phrase_terms(
            "no quotes here",
            &["dashboard".to_string(), "status".to_string()],
        );
        assert!(terms.is_empty());
        let terms = exact_phrase_terms("x", &["is не".to_string(), "a b".to_string()]);
        assert!(terms.is_empty(), "{terms:?}");
    }

    #[test]
    fn exact_phrase_terms_extracts_quoted_spans_from_the_task() {
        let terms = exact_phrase_terms(
            r#"Why does it show "Waiting for status" instead of "Ready to go" now?"#,
            &[],
        );
        assert_eq!(
            terms,
            vec!["Waiting for status".to_string(), "Ready to go".to_string()]
        );
    }

    #[test]
    fn exact_phrase_terms_ignores_an_unclosed_quote() {
        let terms = exact_phrase_terms(r#"look for "Waiting for status"#, &[]);
        assert!(terms.is_empty(), "{terms:?}");
    }

    #[test]
    fn exact_phrase_terms_dedupes_and_caps() {
        let terms = exact_phrase_terms(
            r#"see "Waiting for status" please"#,
            &["Waiting for status".to_string()],
        );
        assert_eq!(terms.len(), 1);
        let many: Vec<String> = (0..40).map(|i| format!("phrase number {i}")).collect();
        assert_eq!(exact_phrase_terms("x", &many).len(), MAX_EXACT_PHRASES);
    }

    #[test]
    fn test_filters_stop_words() {
        let symbols = extract_symbols_from_query("the is in for to a an");
        assert!(symbols.is_empty());
    }

    #[test]
    fn conceptual_terms_keep_four_letters_and_more_without_the_stop_words() {
        assert_eq!(
            conceptual_query_terms("find the retry loop of sync", &["Backoff".into()]),
            vec!["retry", "loop", "sync", "backoff"]
        );
    }

    fn make_search_result(name: &str, file_path: &str, score: f64) -> Scored {
        Scored {
            node: Node {
                id: name.bytes().map(i64::from).sum(),
                name: name.into(),
                kind: "function".into(),
                parent: String::new(),
                path: file_path.into(),
                line: 1,
                end_line: 5,
                sig: String::new(),
                doc: String::new(),
                vis: "pub".into(),
            },
            score,
        }
    }

    #[test]
    fn test_cooccurrence_boost_multi_term() {
        let mut candidates = vec![
            make_search_result("auth_handler", "src/auth.rs", 10.0),
            make_search_result("user_list", "src/user.rs", 10.0),
        ];
        let terms = vec!["auth".to_string(), "handler".to_string()];
        apply_cooccurrence_boost(&mut candidates, &terms);
        assert_eq!(candidates[0].node.name, "auth_handler");
        assert!(candidates[0].score > candidates[1].score);
    }

    #[test]
    fn test_cooccurrence_no_boost_single_term() {
        let mut candidates = vec![make_search_result("auth", "src/auth.rs", 10.0)];
        let terms = vec!["auth".to_string(), "handler".to_string()];
        apply_cooccurrence_boost(&mut candidates, &terms);
        assert_eq!(candidates[0].score, 10.0);
    }

    #[test]
    fn test_per_file_cap_limits_single_file() {
        let candidates = vec![
            make_search_result("fn1", "src/big.rs", 10.0),
            make_search_result("fn2", "src/big.rs", 9.0),
            make_search_result("fn3", "src/big.rs", 8.0),
            make_search_result("fn4", "src/other.rs", 7.0),
        ];
        let result = apply_per_file_cap(candidates, 10, 2);
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].name, "fn1");
        assert_eq!(result[1].name, "fn2");
        assert_eq!(result[2].name, "fn4");
        assert_eq!(result[3].name, "fn3");
    }

    #[test]
    fn test_per_file_cap_respects_max_total() {
        let candidates = vec![
            make_search_result("fn1", "src/a.rs", 10.0),
            make_search_result("fn2", "src/b.rs", 9.0),
            make_search_result("fn3", "src/c.rs", 8.0),
        ];
        let result = apply_per_file_cap(candidates, 2, 5);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_per_file_cap_reserves_executable_owner() {
        let mut import = make_search_result("solver_import", "src/big.rs", 10.0);
        import.node.kind = "use".into();
        let mut config = make_search_result("SolverConfig", "src/big.rs", 9.0);
        config.node.kind = "struct".into();
        let owner = make_search_result("run_solver", "src/big.rs", 8.0);
        let other = make_search_result("helper", "src/other.rs", 7.0);
        let result = apply_per_file_cap(vec![import, config, owner, other], 3, 2);
        let names: Vec<_> = result.iter().map(|node| node.name.as_str()).collect();
        assert!(names.contains(&"run_solver"), "selected roots: {names:?}");
        assert!(names.contains(&"helper"), "selected roots: {names:?}");
    }

    fn node_at(line: u32, end_line: u32) -> Node {
        let mut n = make_search_result("wide", "src/wide.rs", 1.0).node;
        n.line = line;
        n.end_line = end_line;
        n
    }

    // Ported from tokensave `src/context/builder.rs`'s tests: the handle names `index_body`.

    #[test]
    fn test_truncated_code_block_points_at_index_body() {
        let code = "fn wide() {\n".to_string() + &"    line();\n".repeat(50);
        let out = truncate_code_block(&code, 60, None, &node_at(1, 51));
        let handle = format!("index_body node_id={}", node_at(1, 51).id);
        assert!(
            out.contains(&handle),
            "truncated block must carry a follow-up handle: {out}"
        );
        assert!(out.starts_with("fn wide() {"), "{out}");
        let body = out.split("\n... [").next().unwrap();
        assert!(!body.ends_with("    line"), "cut mid-line: {body:?}");
    }

    #[test]
    fn test_short_code_block_is_returned_verbatim() {
        let code = "fn small() {}\n";
        assert_eq!(truncate_code_block(code, 1500, None, &node_at(1, 1)), code);
        assert_eq!(
            truncate_code_block(code, code.len(), None, &node_at(1, 1)),
            code
        );
    }

    #[test]
    fn test_truncate_code_block_does_not_split_multibyte_char() {
        let code = "fn f() {\n    // ✅✅✅✅✅✅✅✅✅✅\n    body();\n}\n";
        let out = truncate_code_block(code, 20, None, &node_at(1, 4));
        assert!(out.contains("index_body node_id="), "{out}");
    }

    #[test]
    fn test_max_code_lines_caps_the_snippet() {
        let code = "fn wide() {\n".to_string() + &"    line();\n".repeat(50);
        let out = truncate_code_block(&code, usize::MAX, Some(3), &node_at(1, 51));
        let body = out.split("\n... [").next().unwrap();
        assert_eq!(body.lines().count(), 3, "{out}");
        assert!(out.contains("index_body node_id="), "{out}");
    }

    #[test]
    fn test_max_code_lines_leaves_shorter_snippets_verbatim() {
        let code = "fn small() {\n    body();\n}\n";
        assert_eq!(
            truncate_code_block(code, usize::MAX, Some(20), &node_at(1, 3)),
            code,
            "a snippet under the cap must come back untouched"
        );
    }

    #[test]
    fn test_byte_limit_still_applies_under_a_line_cap() {
        let code = "fn wide() {\n".to_string() + &"    line();\n".repeat(50);
        let out = truncate_code_block(&code, 30, Some(10), &node_at(1, 51));
        let body = out.split("\n... [").next().unwrap();
        assert!(body.len() <= 30, "byte cap ignored: {body:?}");
        assert!(!body.ends_with("    line"), "cut mid-line: {body:?}");
    }

    #[test]
    fn line_prefix_boundaries() {
        assert_eq!(line_prefix("a\nb\nc", 0), "a\nb\nc");
        assert_eq!(line_prefix("a\nb\nc", 1), "a");
        assert_eq!(line_prefix("a\nb\n", 2), "a\nb\n");
        assert_eq!(line_prefix("a", 5), "a");
    }

    #[test]
    fn branches_and_loops_are_read_from_the_words() {
        assert_eq!(
            branches_and_loops("if a { for x in y {} } else if b { while c {} }"),
            4
        );
        assert_eq!(branches_and_loops("let iffy = format();"), 0);
    }

    #[test]
    fn the_options_of_the_tool_are_tokensaves() {
        let o = BuildContextOptions::of_tool(None, Vec::new());
        assert_eq!(o.max_nodes, 20);
        assert_eq!(o.max_per_file, Some(6));
        assert_eq!(o.search_limit, 3);
        assert_eq!(o.traversal_depth, 1);
        assert_eq!(o.max_code_blocks, 5);
        assert_eq!(o.max_code_block_size, 1500);
        assert_eq!(
            BuildContextOptions::of_tool(Some(500), Vec::new()).max_nodes,
            100
        );
        assert_eq!(
            BuildContextOptions::of_tool(Some(6), Vec::new()).max_per_file,
            Some(3)
        );
    }
}
