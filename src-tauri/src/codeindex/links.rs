//! The links between symbols (ESC-297): the references of the files (`edges`) resolved to the
//! symbols they mean (`links`, from an id to an id), the ties kept apart (`ambiguous`), at each
//! reading and only for what it could have changed.
//!
//! The incremental part is ported from tokensave (github.com/aovestdipaperino/tokensave, commit
//! 580936b, MIT, © 2025 Enzo Lombardi), `src/resolution/touched.rs`: the references to resolve
//! again are those of the files read again, and those whose name is a key of a symbol that came or
//! went. Everything else resolves as it did.
//!
//! The worktrees are ours (`docs/CODEINDEX-PROBES.md`, § IDX-2): a worktree's links are its own
//! files' references, and those of the project's files whose resolution the worktree's delta could
//! change (the same touched set, from the symbols it hides and those it has), resolved against what
//! the worktree sees; the project's links for these are hidden in it (`overrides`). They are made
//! again when the worktree or the project changed.

use super::resolver::{Ambiguous, Import, Node, Resolved, Resolver, Touched, Uref};
use anyhow::Result;
use rusqlite::{params, Connection, Transaction};
use std::collections::HashSet;

/// tokensave's qualified name: the file, the holders, the name (`src/a.rs::Type::method`).
pub fn qname(path: &str, parent: &str, name: &str) -> String {
    let mut q = path.to_string();
    for p in parent.split('.').filter(|p| !p.is_empty()) {
        q.push_str("::");
        q.push_str(p);
    }
    q.push_str("::");
    q.push_str(name);
    q
}

/// The rows of alias `a` the scope `:w` sees (as `api::Reader::vis`).
fn vis(a: &str) -> String {
    format!(
        "({a}.scope = :w OR ({a}.scope = '' AND NOT EXISTS (SELECT 1 FROM files fw \
         WHERE fw.scope = :w AND fw.path = {a}.path AND fw.state IN (1, 2))))"
    )
}

fn nodes(tx: &Transaction, scope: &str) -> Result<Vec<Node>> {
    let sql = format!(
        "SELECT s.id, s.name, s.qname, s.kind, s.path, s.line, s.vis, s.parent, s.ty
         FROM symbols s WHERE s.kind <> 'heading' AND {}",
        vis("s")
    );
    let mut stmt = tx.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::named_params! {":w": scope}, |r| {
        let parent: String = r.get(7)?;
        Ok(Node {
            id: r.get(0)?,
            name: r.get(1)?,
            qname: r.get(2)?,
            kind: r.get(3)?,
            path: r.get(4)?,
            line: r.get(5)?,
            vis: r.get::<_, String>(6)? == "pub",
            parent: parent.rsplit('.').next().unwrap_or("").to_string(),
            ty: r.get(8)?,
        })
    })?;
    Ok(rows.flatten().collect())
}

fn files(tx: &Transaction, scope: &str) -> Result<HashSet<String>> {
    let sql = format!(
        "SELECT f.path FROM files f WHERE f.state <> 2 AND {}",
        vis("f")
    );
    let mut stmt = tx.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::named_params! {":w": scope}, |r| r.get(0))?;
    Ok(rows.flatten().collect())
}

fn imports(tx: &Transaction, scope: &str) -> Result<Vec<(String, String)>> {
    let sql = format!(
        "SELECT e.path, e.target FROM edges e WHERE e.kind = 'import' AND {}",
        vis("e")
    );
    let mut stmt = tx.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::named_params! {":w": scope}, |r| {
        Ok((r.get(0)?, r.get(1)?))
    })?;
    Ok(rows.flatten().collect())
}

/// The references to resolve of the rows of `where_`.
fn urefs(tx: &Transaction, where_: &str, scope: &str) -> Result<Vec<Uref>> {
    let sql = format!(
        "SELECT e.id, e.from_id, e.kind, e.target, e.path, e.line, e.col, e.via FROM edges e
         WHERE e.kind <> 'import' AND {where_}"
    );
    let mut stmt = tx.prepare(&sql)?;
    let named: &[(&str, &dyn rusqlite::ToSql)] = if where_.contains(":w") {
        rusqlite::named_params! {":w": scope}
    } else {
        &[]
    };
    let rows = stmt.query_map(named, |r| {
        Ok(Uref {
            id: r.get(0)?,
            from: r.get(1)?,
            kind: r.get(2)?,
            name: r.get(3)?,
            path: r.get(4)?,
            line: r.get(5)?,
            col: r.get(6)?,
            via: r.get::<_, i64>(7)? as u8,
        })
    })?;
    Ok(rows.flatten().collect())
}

fn store(
    tx: &Transaction,
    scope: &str,
    resolved: &[Resolved],
    ambiguous: &[Ambiguous],
) -> Result<()> {
    let mut link = tx.prepare_cached(
        "INSERT INTO links (ref_id, scope, path, from_id, to_id, kind, line, conf, by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?;
    // Two references of one call site that resolve to the same symbol make one link (tokensave's
    // `create_edges`); the one kept is the first, the order being the references'.
    let mut seen: HashSet<(Option<i64>, i64, String, u32)> = HashSet::new();
    for r in resolved {
        let u = &r.uref;
        if !seen.insert((u.from, r.to, u.kind.clone(), u.line)) {
            continue;
        }
        link.execute(params![
            u.id,
            scope,
            u.path,
            u.from,
            r.to,
            u.kind,
            u.line,
            r.confidence,
            r.by
        ])?;
    }
    let mut amb = tx.prepare_cached(
        "INSERT INTO ambiguous (ref_id, scope, path, from_id, name, line, candidates, simple)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )?;
    let mut seen_amb: HashSet<(Option<i64>, String, u32, u32, String)> = HashSet::new();
    for a in ambiguous {
        let u = &a.uref;
        let simple = super::resolver::simple_ref_name(&u.name).to_string();
        // A call site's references that tie on the same candidates are one ambiguity.
        if !seen_amb.insert((u.from, u.path.clone(), u.line, u.col, simple.clone())) {
            continue;
        }
        let ids: Vec<String> = a.candidates.iter().map(i64::to_string).collect();
        amb.execute(params![
            u.id,
            scope,
            u.path,
            u.from,
            u.name,
            u.line,
            ids.join(","),
            simple
        ])?;
    }
    Ok(())
}

/// What a resolver is made of: the symbols seen, the files, the imports (file, name).
type Inputs = (Vec<Node>, HashSet<String>, Vec<(String, String)>);

fn resolver_inputs(tx: &Transaction, scope: &str) -> Result<Inputs> {
    Ok((nodes(tx, scope)?, files(tx, scope)?, imports(tx, scope)?))
}

/// Resolves again, in the project's folder (`""`), the references `touched` could have changed;
/// everything when `full`.
pub fn resolve_project(tx: &Transaction, touched: &Touched, full: bool) -> Result<()> {
    if !full && touched.is_empty() {
        return Ok(());
    }
    let (nodes, files, imports) = resolver_inputs(tx, "")?;
    let imps: Vec<Import> = imports
        .iter()
        .map(|(p, n)| Import { path: p, name: n })
        .collect();
    let resolver = Resolver::new(&nodes, &files, &imps);
    let todo: Vec<Uref> = if full {
        tx.execute("DELETE FROM links WHERE scope = ''", [])?;
        tx.execute("DELETE FROM ambiguous WHERE scope = ''", [])?;
        urefs(tx, "e.scope = ''", "")?
    } else {
        select(tx, touched, "e.scope = ''")?;
        tx.execute(
            "DELETE FROM links WHERE ref_id IN (SELECT id FROM temp.ci_sel)",
            [],
        )?;
        tx.execute(
            "DELETE FROM ambiguous WHERE ref_id IN (SELECT id FROM temp.ci_sel)",
            [],
        )?;
        urefs(tx, "e.id IN (SELECT id FROM temp.ci_sel)", "")?
    };
    let out = resolver.resolve_all(&todo);
    store(tx, "", &out.resolved, &out.ambiguous)?;
    tx.execute("UPDATE scopes SET gen = gen + 1 WHERE scope = ''", [])?;
    Ok(())
}

/// Puts in `temp.ci_sel` the ids of the references of `where_` (on `edges e`) that `touched` could
/// have changed (`Touched::needs_resolve`, asked of SQLite), with every reference of the same call
/// sites: a typed call and its name-based siblings are always judged together.
fn select(tx: &Transaction, touched: &Touched, where_: &str) -> Result<()> {
    tx.execute_batch(
        "CREATE TEMP TABLE IF NOT EXISTS ci_names (n TEXT PRIMARY KEY);
         CREATE TEMP TABLE IF NOT EXISTS ci_files (p TEXT PRIMARY KEY);
         CREATE TEMP TABLE IF NOT EXISTS ci_sel (id INTEGER PRIMARY KEY);
         DELETE FROM temp.ci_names; DELETE FROM temp.ci_files; DELETE FROM temp.ci_sel;",
    )?;
    {
        let mut n = tx.prepare_cached("INSERT OR IGNORE INTO temp.ci_names (n) VALUES (?1)")?;
        for name in &touched.names {
            n.execute([name])?;
        }
        let mut f = tx.prepare_cached("INSERT OR IGNORE INTO temp.ci_files (p) VALUES (?1)")?;
        for p in &touched.files {
            f.execute([p])?;
        }
    }
    // Each by its index: the file, the simple name (a qualified target ends with the name of the
    // symbol it reaches, whose keys are all touched together), the field of `field::method`.
    // The files touched first (`CROSS JOIN` keeps that order): SQLite would scan the references.
    for (from, cond) in [
        (
            "files f CROSS JOIN edges e ON e.file_id = f.id",
            "f.path IN (SELECT p FROM temp.ci_files)",
        ),
        ("edges e", "e.simple IN (SELECT n FROM temp.ci_names)"),
        (
            "edges e",
            "e.via = 2 AND e.recv IN (SELECT n FROM temp.ci_names)",
        ),
    ] {
        tx.execute(
            &format!(
                "INSERT OR IGNORE INTO temp.ci_sel (id)
                 SELECT e.id FROM {from} WHERE e.kind <> 'import' AND {where_} AND {cond}"
            ),
            [],
        )?;
    }
    tx.execute(
        "INSERT OR IGNORE INTO temp.ci_sel (id)
         SELECT e.id FROM edges s JOIN edges e
           ON e.file_id = s.file_id AND e.line = s.line AND e.col = s.col AND e.kind = 'call'
         WHERE s.id IN (SELECT id FROM temp.ci_sel) AND s.kind = 'call'",
        [],
    )?;
    Ok(())
}

/// Makes a worktree's links again, when it or the project changed since.
pub fn resolve_worktree(tx: &Transaction, scope: &str, changed: bool) -> Result<()> {
    let gen: i64 = tx
        .query_row("SELECT gen FROM scopes WHERE scope = ''", [], |r| r.get(0))
        .unwrap_or(0);
    let base_gen: i64 = tx
        .query_row(
            "SELECT base_gen FROM scopes WHERE scope = ?1",
            [scope],
            |r| r.get(0),
        )
        .unwrap_or(-1);
    if !changed && base_gen == gen {
        return Ok(());
    }
    tx.execute("DELETE FROM links WHERE scope = ?1", [scope])?;
    tx.execute("DELETE FROM ambiguous WHERE scope = ?1", [scope])?;
    tx.execute("DELETE FROM overrides WHERE scope = ?1", [scope])?;
    // What the worktree changes: the project's symbols it hides, its own, their files.
    let mut touched = Touched::default();
    {
        let mut stmt = tx.prepare(
            "SELECT s.name, s.qname, s.path FROM symbols s WHERE s.scope = ?1
             UNION ALL
             SELECT s.name, s.qname, s.path FROM symbols s JOIN files fw
               ON fw.scope = ?1 AND fw.path = s.path AND fw.state IN (1, 2)
             WHERE s.scope = ''",
        )?;
        let rows = stmt.query_map([scope], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        for (name, q, path) in rows.flatten() {
            touched.touch_symbol(&name, &q);
            touched.names.insert(path);
        }
        let mut stmt = tx.prepare("SELECT path FROM files WHERE scope = ?1 AND state IN (1, 2)")?;
        let rows = stmt.query_map([scope], |r| r.get::<_, String>(0))?;
        for p in rows.flatten() {
            touched.names.insert(p.clone());
            touched.files.insert(p);
        }
    }
    let own = urefs(tx, "e.scope = :w", scope)?;
    if own.is_empty() && touched.is_empty() {
        set_base_gen(tx, scope, gen)?;
        return Ok(());
    }
    let (nodes, files, imports) = resolver_inputs(tx, scope)?;
    let imps: Vec<Import> = imports
        .iter()
        .map(|(p, n)| Import { path: p, name: n })
        .collect();
    let resolver = Resolver::new(&nodes, &files, &imps);
    // The scope is ours (`store::scope_of`: `wt-` and hex digits), written in the query.
    debug_assert!(scope.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
    select(
        tx,
        &touched,
        &format!(
            "e.scope = '' AND NOT EXISTS (SELECT 1 FROM files fw WHERE fw.scope = '{scope}' \
             AND fw.path = e.path AND fw.state IN (1, 2))"
        ),
    )?;
    let base = urefs(tx, "e.id IN (SELECT id FROM temp.ci_sel)", "")?;
    {
        let mut ov =
            tx.prepare_cached("INSERT OR IGNORE INTO overrides (scope, ref_id) VALUES (?1, ?2)")?;
        for u in &base {
            ov.execute(params![scope, u.id])?;
        }
    }
    let mut todo = own;
    todo.extend(base);
    let out = resolver.resolve_all(&todo);
    store(tx, scope, &out.resolved, &out.ambiguous)?;
    set_base_gen(tx, scope, gen)?;
    Ok(())
}

fn set_base_gen(tx: &Transaction, scope: &str, gen: i64) -> Result<()> {
    tx.execute(
        "UPDATE scopes SET base_gen = ?2 WHERE scope = ?1",
        params![scope, gen],
    )?;
    Ok(())
}

/// Every link of a scope made again from nothing, for the tests that hold the incremental resolution
/// to a full one.
pub fn resolve_all_again(conn: &mut Connection, scope: &str) -> Result<()> {
    let tx = conn.transaction()?;
    if scope.is_empty() {
        resolve_project(&tx, &Touched::default(), true)?;
    } else {
        resolve_worktree(&tx, scope, true)?;
    }
    tx.commit()?;
    Ok(())
}
