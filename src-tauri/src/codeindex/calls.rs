//! Who calls a symbol, what it calls, what a change to it would touch (ESC-297): read from the
//! resolved links (`links`), from an id to an id, never from a name.
//!
//! The way a name is answered is ported from tokensave (github.com/aovestdipaperino/tokensave,
//! commit 580936b, MIT, © 2025 Enzo Lombardi), `src/mcp/tools/handlers/symbol_target.rs`
//! (`resolve_symbol_nodes`, `member_path_segments`, `body_kind_preference`, `KindRanking::Definition`,
//! `resolve_target`, `MAX_MATCHES_DEFAULT`) and `src/db/queries/edges.rs`
//! (`get_nodes_by_qualified_name`): an id answers that symbol; a name answers each definition it
//! names in its own group, the best kinds only, overloads together, ordered as the search ranks
//! them, up to `max_matches`, the others listed without results. `path` keeps the definitions of a
//! file. The calls left to a tie are told apart (`ambiguous`).

use super::api::{Answer, Hit, Reader};
use anyhow::Result;
use rusqlite::{named_params, Row};
use serde::Serialize;
use std::collections::{BTreeSet, HashMap, HashSet};

/// Matches answered when a name names several definitions (`MAX_MATCHES_DEFAULT`, `_MAX`).
pub const MAX_MATCHES_DEFAULT: usize = 3;
pub const MAX_MATCHES_MAX: usize = 20;

/// What a tool is asked about: a symbol by its id, or by a name (and the file it is in).
#[derive(Debug, Clone, Copy)]
pub enum Wanted<'a> {
    Id(i64),
    Name {
        name: &'a str,
        path: Option<&'a str>,
        max_matches: usize,
    },
}

/// Where a symbol is used.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Ref {
    /// The symbol that holds the call; none at the top of a file.
    pub id: Option<i64>,
    pub name: String,
    pub kind: String,
    pub parent: String,
    pub path: String,
    /// The line of the call.
    pub line: u32,
    /// 1 for a direct caller, more for those that call them.
    pub depth: u32,
}

/// A call left unresolved by a tie: the evidence did not part its candidates.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Tie {
    /// The symbol that holds the call.
    pub name: String,
    pub path: String,
    pub line: u32,
    /// The call, as written.
    pub called: String,
    /// How many symbols it may mean.
    pub candidates: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallersOf {
    pub def: Hit,
    pub callers: Vec<Ref>,
    /// Callers in all, `callers` holding at most the limit.
    pub total: usize,
    /// The calls that may mean it, among others.
    pub ambiguous: Vec<Tie>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalleesOf {
    pub def: Hit,
    pub calls: Vec<Hit>,
    pub ambiguous: Vec<Tie>,
    /// Called but defined out of the project (the language's, libraries').
    pub external: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpactOf {
    /// What the change is about: the definition, or the symbols of the file.
    pub targets: Vec<Hit>,
    pub callers: Vec<Ref>,
    /// Types that extend or implement it.
    pub subtypes: Vec<Ref>,
    /// Files that import it, by name.
    pub importers: Vec<String>,
    /// Every file touched.
    pub files: Vec<String>,
    /// The calls that may mean it, among others.
    pub ambiguous: Vec<Tie>,
}

/// An answer for each definition asked about.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Matches<T> {
    /// What was asked: a name, or `#id`.
    pub symbol: String,
    pub matches: Vec<T>,
    /// The other definitions of the name, without results: ask again by their id.
    pub more: Vec<Hit>,
}

/// `body_kind_preference` read through `KindRanking::Definition`: a function and a type rank
/// together, ahead of an `impl`, a constant, a field.
fn rank(kind: &str) -> u8 {
    let pref = match kind {
        "function" | "method" => 0,
        "struct" | "enum" | "trait" | "class" | "interface" | "type" | "component" => 1,
        "impl" => 2,
        "const" | "macro" => 3,
        "field" => 4,
        _ => 6,
    };
    pref.max(1)
}

/// `member_path_segments`: `A.b`, `A::b` are `[A, b]`.
fn segments(name: &str) -> Vec<&str> {
    name.split("::")
        .flat_map(|p| p.split('.'))
        .filter(|s| !s.is_empty())
        .collect()
}

const HIT: &str =
    "s.id, s.name, s.kind, s.parent, s.path, s.line, s.end_line, s.sig, s.doc, s.qname";

impl Reader {
    /// The links alias `a` (of `links` or `ambiguous`) this folder sees: the project's, but for the
    /// files the worktree changed or removed and the references it resolved again, and its own.
    fn lvis(a: &str) -> String {
        format!(
            "({a}.scope = :w OR ({a}.scope = '' AND NOT EXISTS (SELECT 1 FROM files fw \
             WHERE fw.scope = :w AND fw.path = {a}.path AND fw.state IN (1, 2)) \
             AND NOT EXISTS (SELECT 1 FROM overrides o WHERE o.scope = :w AND o.ref_id = {a}.ref_id)))"
        )
    }

    /// `lvis` with the scope as the positional parameter `?n`.
    pub(super) fn lvis_pos(a: &str, n: usize) -> String {
        format!(
            "({a}.scope = ?{n} OR ({a}.scope = '' AND NOT EXISTS (SELECT 1 FROM files fw \
             WHERE fw.scope = ?{n} AND fw.path = {a}.path AND fw.state IN (1, 2)) \
             AND NOT EXISTS (SELECT 1 FROM overrides o WHERE o.scope = ?{n} AND o.ref_id = {a}.ref_id)))"
        )
    }

    fn row_hit(r: &Row) -> rusqlite::Result<(Hit, String)> {
        Ok((Self::hit(r)?, r.get("qname")?))
    }

    /// A symbol this folder sees, by its id.
    pub(super) fn by_id(&self, id: i64) -> Result<Option<Hit>> {
        let sql = format!(
            "SELECT {HIT} FROM symbols s WHERE s.id = :id AND {}",
            Self::vis("s")
        );
        Ok(self
            .conn
            .query_row(&sql, named_params! {":id": id, ":w": self.scope}, |r| {
                Self::hit(r)
            })
            .ok())
    }

    fn by_sql(&self, cond: &str, arg: &str) -> Result<Vec<(Hit, String)>> {
        let sql = format!(
            "SELECT {HIT} FROM symbols s WHERE {cond} AND s.kind <> 'heading' AND {}
             ORDER BY s.path, s.line LIMIT 50",
            Self::vis("s")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(named_params! {":a": arg, ":w": self.scope}, Self::row_hit)?;
        Ok(rows.flatten().collect())
    }

    /// `resolve_symbol_nodes`: the qualified name; else its tail (`Type::method`) or the name; else
    /// the members whose path ends with it (`Type.method`).
    fn symbol_nodes(&self, symbol: &str) -> Result<Vec<(Hit, String)>> {
        let literal = self.by_sql("s.qname = :a", symbol)?;
        if !literal.is_empty() {
            return Ok(literal);
        }
        let fallback = if symbol.contains("::") {
            let esc = symbol
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            self.by_sql("s.qname LIKE '%::' || :a ESCAPE '\\'", &esc)?
        } else {
            self.by_sql("s.name = :a", symbol)?
        };
        if !fallback.is_empty() {
            return Ok(fallback);
        }
        let wanted = segments(symbol);
        let Some(last) = wanted.last().filter(|_| wanted.len() >= 2) else {
            return Ok(Vec::new());
        };
        Ok(self
            .by_sql("s.name = :a", last)?
            .into_iter()
            .filter(|(_, q)| segments(q).ends_with(&wanted))
            .collect())
    }

    /// The definitions a tool answers, each group the overloads of one qualified name; the others.
    fn targets(&self, wanted: Wanted) -> Result<(String, Vec<Vec<Hit>>, Vec<Hit>)> {
        let (name, path, max) = match wanted {
            Wanted::Id(id) => {
                let groups = self.by_id(id)?.map(|h| vec![vec![h]]).unwrap_or_default();
                return Ok((format!("#{id}"), groups, Vec::new()));
            }
            Wanted::Name {
                name,
                path,
                max_matches,
            } => (name.trim(), path, max_matches.clamp(1, MAX_MATCHES_MAX)),
        };
        let mut nodes = self.symbol_nodes(name)?;
        if let Some(p) = path.map(|p| p.replace('\\', "/")) {
            nodes.retain(|(h, _)| h.path == p || h.path.ends_with(&format!("/{p}")));
        }
        let mut seen = HashSet::new();
        nodes.retain(|(h, _)| seen.insert(h.id));
        if let Some(best) = nodes.iter().map(|(h, _)| rank(&h.kind)).min() {
            nodes.retain(|(h, _)| rank(&h.kind) == best);
        }
        let mut groups: Vec<(String, Vec<Hit>)> = Vec::new();
        for (h, q) in nodes {
            match groups.iter_mut().find(|(gq, _)| *gq == q) {
                Some((_, g)) => g.push(h),
                None => groups.push((q, vec![h])),
            }
        }
        if groups.len() > 1 {
            let ranks: HashMap<i64, usize> = self
                .search(name, 100, None)?
                .result
                .into_iter()
                .enumerate()
                .map(|(i, h)| (h.id, i))
                .collect();
            groups.sort_by_cached_key(|(_, g)| {
                let r = g
                    .iter()
                    .filter_map(|h| ranks.get(&h.id).copied())
                    .min()
                    .unwrap_or(usize::MAX);
                (r, g[0].path.clone(), g[0].line)
            });
        }
        let mut groups: Vec<Vec<Hit>> = groups.into_iter().map(|(_, g)| g).collect();
        let rest = groups.split_off(groups.len().min(max));
        Ok((
            name.to_string(),
            groups,
            rest.into_iter().flatten().collect(),
        ))
    }

    fn caller_rows(&self, id: i64, kinds: &str, limit: usize) -> Result<Vec<Ref>> {
        let sql = format!(
            "SELECT l.line AS cline, l.path AS cpath, l.from_id, f.name, f.kind, f.parent
             FROM links l LEFT JOIN symbols f ON f.id = l.from_id
             WHERE l.to_id = :id AND l.kind IN {kinds} AND {}
             ORDER BY l.path, l.line LIMIT :n",
            Self::lvis("l")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(
            named_params! {":id": id, ":w": self.scope, ":n": limit as i64},
            |r| {
                Ok(Ref {
                    id: r.get("from_id")?,
                    name: r.get::<_, Option<String>>("name")?.unwrap_or_default(),
                    kind: r.get::<_, Option<String>>("kind")?.unwrap_or_default(),
                    parent: r.get::<_, Option<String>>("parent")?.unwrap_or_default(),
                    path: r.get("cpath")?,
                    line: r.get("cline")?,
                    depth: 1,
                })
            },
        )?;
        Ok(rows.flatten().collect())
    }

    /// The direct callers of a symbol, by its id.
    pub(super) fn callers_of_id(&self, id: i64, limit: usize) -> Result<Vec<Ref>> {
        self.caller_rows(id, "('call')", limit)
    }

    /// The definitions of the direct callers of a symbol, by its id.
    pub(super) fn caller_hits(&self, id: i64, limit: usize) -> Result<Vec<Hit>> {
        let mut out: Vec<Hit> = Vec::new();
        for r in self.callers_of_id(id, limit * 3)? {
            let Some(from) = r.id else { continue };
            if let Some(h) = self.by_id(from)? {
                if !out.contains(&h) {
                    out.push(h);
                }
            }
            if out.len() >= limit {
                break;
            }
        }
        Ok(out)
    }

    fn count_callers(&self, id: i64) -> Result<usize> {
        let sql = format!(
            "SELECT COUNT(*) FROM links l WHERE l.to_id = :id AND l.kind = 'call' AND {}",
            Self::lvis("l")
        );
        let n: i64 =
            self.conn
                .query_row(&sql, named_params! {":id": id, ":w": self.scope}, |r| {
                    r.get(0)
                })?;
        Ok(n as usize)
    }

    fn ties(&self, cond: &str, id: i64, limit: usize) -> Result<Vec<Tie>> {
        let sql = format!(
            "SELECT a.path, a.line, a.name AS called, a.candidates, f.name
             FROM ambiguous a LEFT JOIN symbols f ON f.id = a.from_id
             WHERE {cond} AND {} ORDER BY a.path, a.line LIMIT :n",
            Self::lvis("a")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(
            named_params! {":id": id, ":w": self.scope, ":n": limit as i64},
            |r| {
                let c: String = r.get("candidates")?;
                Ok(Tie {
                    name: r.get::<_, Option<String>>("name")?.unwrap_or_default(),
                    path: r.get("path")?,
                    line: r.get("line")?,
                    called: r.get("called")?,
                    candidates: c.split(',').filter(|s| !s.is_empty()).count(),
                })
            },
        )?;
        Ok(rows.flatten().collect())
    }

    /// The ties a symbol is among the candidates of.
    fn ties_to(&self, id: i64, limit: usize) -> Result<Vec<Tie>> {
        // Every candidate bears the call's simple name: the index finds the few ties of that name.
        self.ties(
            "a.simple = (SELECT s.name FROM symbols s WHERE s.id = :id) \n             AND (',' || a.candidates || ',') LIKE '%,' || :id || ',%'",
            id,
            limit,
        )
    }

    /// What a symbol calls, resolved, by its id: the definitions, in the order of the calls.
    pub(super) fn callees_of_id(&self, id: i64, limit: usize) -> Result<Vec<Hit>> {
        let sql = format!(
            "SELECT l.to_id, MIN(l.line) AS first FROM links l
             WHERE l.from_id = :id AND l.kind = 'call' AND {}
             GROUP BY l.to_id ORDER BY first LIMIT :n",
            Self::lvis("l")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let ids: Vec<i64> = stmt
            .query_map(
                named_params! {":id": id, ":w": self.scope, ":n": limit as i64},
                |r| r.get(0),
            )?
            .flatten()
            .collect();
        let mut out = Vec::new();
        for t in ids {
            if t == id {
                continue;
            }
            if let Some(h) = self.by_id(t)? {
                out.push(h);
            }
        }
        Ok(out)
    }

    /// The names a symbol calls that resolved to nothing and tied with nothing.
    fn external_of(&self, id: i64, limit: usize) -> Result<Vec<String>> {
        let sql = format!(
            "SELECT e.target, e.line, e.col,
                    EXISTS (SELECT 1 FROM links l WHERE l.ref_id = e.id AND {}) AS done,
                    EXISTS (SELECT 1 FROM ambiguous a WHERE a.ref_id = e.id AND {}) AS tied
             FROM edges e WHERE e.from_id = :id AND e.kind = 'call' AND {} ORDER BY e.line",
            Self::lvis("l"),
            Self::lvis("a"),
            Self::vis("e")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows: Vec<(String, u32, u32, bool, bool)> = stmt
            .query_map(named_params! {":id": id, ":w": self.scope}, |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })?
            .flatten()
            .collect();
        // A call site is answered when one of its references is.
        let answered: HashSet<(u32, u32)> = rows
            .iter()
            .filter(|r| r.3 || r.4)
            .map(|r| (r.1, r.2))
            .collect();
        let mut out: BTreeSet<String> = BTreeSet::new();
        for (name, line, col, ..) in rows {
            if answered.contains(&(line, col)) {
                continue;
            }
            out.insert(super::resolver::simple_ref_name(&name).trim().to_string());
            if out.len() >= limit {
                break;
            }
        }
        Ok(out.into_iter().collect())
    }

    /// Who calls a symbol: for each definition asked about, its callers by the resolved links, the
    /// calls that may mean it among others apart.
    pub fn callers(&self, wanted: Wanted, limit: usize) -> Result<Answer<Matches<CallersOf>>> {
        let (symbol, groups, more) = self.targets(wanted)?;
        let mut matches = Vec::new();
        let mut paths: Vec<String> = Vec::new();
        for g in groups {
            for def in g {
                let callers = self.callers_of_id(def.id, limit)?;
                let total = self.count_callers(def.id)?;
                let ambiguous = self.ties_to(def.id, limit)?;
                paths.push(def.path.clone());
                paths.extend(callers.iter().map(|c| c.path.clone()));
                matches.push(CallersOf {
                    def,
                    callers,
                    total,
                    ambiguous,
                });
            }
        }
        let result = Matches {
            symbol,
            matches,
            more,
        };
        let saving = self.saving(&paths, &result);
        Ok(Answer { result, saving })
    }

    /// What a symbol calls: for each definition, the definitions of its calls, the ties, the names
    /// of what is not in the project.
    pub fn callees(&self, wanted: Wanted, limit: usize) -> Result<Answer<Matches<CalleesOf>>> {
        let (symbol, groups, more) = self.targets(wanted)?;
        let mut matches = Vec::new();
        let mut paths: Vec<String> = Vec::new();
        for g in groups {
            for def in g {
                let calls = self.callees_of_id(def.id, limit)?;
                let ambiguous = self.ties("a.from_id = :id", def.id, limit)?;
                let external = self.external_of(def.id, limit)?;
                paths.push(def.path.clone());
                paths.extend(calls.iter().map(|c| c.path.clone()));
                matches.push(CalleesOf {
                    def,
                    calls,
                    ambiguous,
                    external,
                });
            }
        }
        let result = Matches {
            symbol,
            matches,
            more,
        };
        let saving = self.saving(&paths, &result);
        Ok(Answer { result, saving })
    }

    /// The members of a type, with it: a change to the type touches the callers of its methods.
    fn with_members(&self, def: &Hit) -> Result<Vec<Hit>> {
        let mut out = vec![def.clone()];
        if matches!(
            def.kind.as_str(),
            "struct" | "enum" | "trait" | "class" | "interface" | "type" | "impl"
        ) {
            let parent = if def.parent.is_empty() {
                def.name.clone()
            } else {
                format!("{}.{}", def.parent, def.name)
            };
            let sql = format!(
                "SELECT {HIT} FROM symbols s WHERE s.path = :p AND s.parent = :parent
                 AND s.kind IN ('method', 'function') AND {} ORDER BY s.line",
                Self::vis("s")
            );
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map(
                named_params! {":p": def.path, ":parent": parent, ":w": self.scope},
                Self::hit,
            )?;
            out.extend(rows.flatten());
        }
        Ok(out)
    }

    /// What a change to a symbol (by its id or name) or to a file (by its path) would touch: the
    /// callers, up to `depth` calls away, by the resolved links; the types built on it; the files
    /// that import it; the calls that may mean it, apart.
    pub fn impact(
        &self,
        wanted: Wanted,
        depth: u32,
        limit: usize,
    ) -> Result<Answer<Matches<ImpactOf>>> {
        let mut file_path = None;
        if let Wanted::Name {
            name, path: None, ..
        } = wanted
        {
            file_path = self.file_like(name)?;
        }
        let (symbol, sets, more): (String, Vec<Vec<Hit>>, Vec<Hit>) = match &file_path {
            Some(p) => {
                let sql = format!(
                    "SELECT {HIT} FROM symbols s WHERE s.path = :p AND s.kind <> 'heading' AND {}
                     ORDER BY s.line",
                    Self::vis("s")
                );
                let mut stmt = self.conn.prepare(&sql)?;
                let all: Vec<Hit> = stmt
                    .query_map(named_params! {":p": p, ":w": self.scope}, Self::hit)?
                    .flatten()
                    .collect();
                (p.clone(), vec![all], Vec::new())
            }
            None => {
                let (s, groups, more) = self.targets(wanted)?;
                let mut sets = Vec::new();
                for g in groups {
                    for def in g {
                        sets.push(self.with_members(&def)?);
                    }
                }
                (s, sets, more)
            }
        };
        let mut matches = Vec::new();
        let mut paths: Vec<String> = Vec::new();
        for targets in sets {
            let m = self.impact_of(targets, file_path.as_deref(), depth, limit)?;
            paths.extend(m.files.iter().cloned());
            matches.push(m);
        }
        let result = Matches {
            symbol,
            matches,
            more,
        };
        let saving = self.saving(&paths, &result);
        Ok(Answer { result, saving })
    }

    fn impact_of(
        &self,
        targets: Vec<Hit>,
        file: Option<&str>,
        depth: u32,
        limit: usize,
    ) -> Result<ImpactOf> {
        let own_files: HashSet<String> = targets.iter().map(|t| t.path.clone()).collect();
        let mut seen: HashSet<i64> = targets.iter().map(|t| t.id).collect();
        let mut frontier: Vec<i64> = targets.iter().map(|t| t.id).collect();
        let mut callers: Vec<Ref> = Vec::new();
        for d in 1..=depth.max(1) {
            let mut next = Vec::new();
            for id in &frontier {
                for mut r in self.callers_of_id(*id, 200)? {
                    // A call from the file being changed is part of the change.
                    if file.is_some() && own_files.contains(&r.path) {
                        continue;
                    }
                    r.depth = d;
                    if let Some(from) = r.id {
                        if seen.insert(from) {
                            next.push(from);
                        }
                    }
                    if callers.len() < limit
                        && !callers
                            .iter()
                            .any(|c| c.id == r.id && c.path == r.path && c.line == r.line)
                    {
                        callers.push(r);
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            frontier = next;
        }
        let mut subtypes: Vec<Ref> = Vec::new();
        let mut ambiguous: Vec<Tie> = Vec::new();
        for t in &targets {
            for r in self.caller_rows(t.id, "('extends', 'implements')", 50)? {
                if !subtypes.contains(&r) {
                    subtypes.push(r);
                }
            }
            if ambiguous.len() < limit {
                ambiguous.extend(self.ties_to(t.id, limit - ambiguous.len())?);
            }
        }
        // Who imports it: by the name of the symbol, or of the file's module.
        let mut importers: BTreeSet<String> = BTreeSet::new();
        let needles: Vec<String> = match file {
            Some(p) => {
                let stem = p.rsplit('/').next().unwrap_or(p);
                let stem = stem.rsplit_once('.').map(|s| s.0).unwrap_or(stem);
                vec![stem.to_string()]
            }
            None => targets
                .first()
                .map(|t| t.name.clone())
                .into_iter()
                .collect(),
        };
        for needle in needles.iter().filter(|n| n.chars().count() > 2) {
            let sql = format!(
                "SELECT DISTINCT e.path FROM edges e
                 WHERE e.kind = 'import' AND instr(lower(e.target), lower(:n)) > 0 AND {} LIMIT 60",
                Self::vis("e")
            );
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map(named_params! {":n": needle, ":w": self.scope}, |r| {
                r.get::<_, String>(0)
            })?;
            importers.extend(rows.flatten().filter(|p| !own_files.contains(p)));
        }
        let mut files: BTreeSet<String> = BTreeSet::new();
        files.extend(own_files.iter().cloned());
        files.extend(callers.iter().map(|c| c.path.clone()));
        files.extend(subtypes.iter().map(|c| c.path.clone()));
        files.extend(importers.iter().cloned());
        Ok(ImpactOf {
            targets,
            callers,
            subtypes,
            importers: importers.into_iter().collect(),
            files: files.into_iter().collect(),
            ambiguous,
        })
    }
}
