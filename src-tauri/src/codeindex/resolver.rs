//! The resolution of references to symbols (ESC-297): a call, an `extends` or an `implements` written
//! as a name is bound to the one symbol it means, or to none; a tie is never guessed but kept apart,
//! with its candidates.
//!
//! Ported from tokensave (github.com/aovestdipaperino/tokensave, commit 580936b, MIT, © 2025 Enzo
//! Lombardi), `src/resolution/resolver.rs` (`ReferenceResolver`: `from_nodes`, `resolve_one`,
//! `resolve_all`, `try_qualified_match`, `try_go_selector_match`, `try_exact_name_match`,
//! `try_exact_name_match_simple`, `score_candidate`, `explain_ambiguity`, `find_best_matches`,
//! `kind_compatible`, `resolve_from_filtered_named`, `bare_name_needs_evidence`,
//! `is_plausibly_reachable`, `CROSS_FILE_BLOCKLIST`, `path_proximity`, `lang_from_path`,
//! `suppress_go_selector_bare_siblings`, `suppress_gdscript_typed_siblings`), `src/go_import.rs`
//! (`import_identifier`) and `src/resolution/js_specifier.rs` (`relative_module_candidates`). Same
//! steps, same constants, same thresholds.
//!
//! What differs, and why, is written in `docs/CODEINDEX-PROBES.md` (§ IDX-2): the strategies of the
//! languages Escouade does not read (Ruby, GDScript, Terraform, VHDL, C++ headers) are left out;
//! C#'s typed receivers are not ported (its calls go through the common path); a Svelte component is
//! a callable target (its tags call it); `field::method` (`via` 2) is ours; the suppression of a
//! typed call's siblings, tokensave's for GDScript, holds for every language.

use std::collections::{HashMap, HashSet};

/// Names too common to resolve across files reliably (`CROSS_FILE_BLOCKLIST`).
pub const CROSS_FILE_BLOCKLIST: &[&str] = &[
    // Rust std types / prelude
    "Result",
    "Option",
    "String",
    "Vec",
    "Box",
    "Arc",
    "Rc",
    "Ok",
    "Err",
    "Some",
    "None",
    // Ubiquitous trait methods
    "fmt",
    "format",
    "display",
    "to_string",
    "clone",
    "clone_from",
    "default",
    "from",
    "into",
    "try_from",
    "try_into",
    "new",
    "build",
    "builder",
    "parse",
    "from_str",
    "eq",
    "ne",
    "cmp",
    "partial_cmp",
    "hash",
    "next",
    "iter",
    "into_iter",
    "drop",
    "deref",
    "deref_mut",
    "as_ref",
    "as_mut",
    "borrow",
    "borrow_mut",
    "read",
    "write",
    "flush",
    "close",
    "len",
    "is_empty",
    "contains",
    "push",
    "pop",
    "insert",
    "remove",
    "get",
    "unwrap",
    "expect",
    "map",
    "and_then",
    "or_else",
    "unwrap_or",
    // Common test/assertion names
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    // Common patterns matched across files
    "run",
    "start",
    "stop",
    "init",
    "setup",
    // Stdlib method names that collide with user-defined functions
    "status",
    "modified",
    "output",
    "exists",
    "join",
    "display",
    "to_owned",
    "collect",
    "filter",
    "find",
    "take",
    "skip",
    "count",
    "sum",
    "max",
    "min",
    "sort",
    "extend",
    "chain",
    "zip",
    "enumerate",
    "flatten",
    "open",
    "create",
    "metadata",
    "canonicalize",
    "spawn",
    "wait",
    "send",
    "recv",
    "lock",
    "try_lock",
];

/// The least confidence a resolution is kept at (`resolve_batch_inner`).
pub const CONFIDENCE_FLOOR: f64 = 0.6;

pub const SIMPLE_NAME_MATCH: &str = "simple-name-match";
pub const PATH_TAIL_MATCH: &str = "path-tail-match";
pub const QUALIFIED_MATCH: &str = "qualified-match";
/// Ours: `field::method`, through the type of the field.
pub const FIELD_TYPED: &str = "field-typed-receiver";

/// A symbol, as the resolver knows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: i64,
    pub name: String,
    /// tokensave's form: `path::Holder::name`.
    pub qname: String,
    pub kind: String,
    pub path: String,
    pub line: u32,
    pub vis: bool,
    /// The name of what holds it (the type of a method), empty at the top.
    pub parent: String,
    /// A field's type (Rust), for `field::method`.
    pub ty: String,
}

/// A reference to resolve.
#[derive(Debug, Clone, PartialEq)]
pub struct Uref {
    pub id: i64,
    pub from: Option<i64>,
    /// `call`, `extends` or `implements`.
    pub kind: String,
    pub name: String,
    pub path: String,
    pub line: u32,
    pub col: u32,
    pub via: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub uref: Uref,
    pub to: i64,
    pub confidence: f64,
    pub by: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ambiguous {
    pub uref: Uref,
    /// Ordered by id.
    pub candidates: Vec<i64>,
}

#[derive(Debug, Default)]
pub struct Outcome {
    pub resolved: Vec<Resolved>,
    pub ambiguous: Vec<Ambiguous>,
    /// Neither resolved nor tied.
    pub unresolved: usize,
}

/// `simple_ref_name`: `Self::watermark_band` is `watermark_band`, `obj.render` is `render`.
pub fn simple_ref_name(name: &str) -> &str {
    let after_path = name.rsplit("::").next().unwrap_or(name);
    after_path.rsplit('.').next().unwrap_or(after_path)
}

/// `lang_from_path`, for the languages Escouade reads (the others are `unknown` here as there).
pub fn lang_from_path(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "rs" => "rust",
        "go" => "go",
        "py" | "pyi" => "python",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "ts" | "tsx" | "mts" | "cts" => "typescript",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "swift" => "swift",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" | "inl" | "ipp" | "tcc" => "cpp",
        "cs" => "csharp",
        "rb" | "rake" | "erb" | "slim" => "ruby",
        "php" => "php",
        "scala" | "sc" => "scala",
        "dart" => "dart",
        "lua" => "lua",
        _ => "unknown",
    }
}

fn same_language_family(a: &str, b: &str) -> bool {
    a == b || matches!((a, b), ("c", "cpp") | ("cpp", "c"))
}

/// `path_proximity`: +5 per shared leading path segment, capped at +40.
pub fn path_proximity(a: &str, b: &str) -> i64 {
    let shared = a
        .split('/')
        .zip(b.split('/'))
        .take_while(|(x, y)| x == y)
        .count();
    (shared as i64 * 5).min(40)
}

/// `go_file_in_package`.
fn go_file_in_package(file_path: &str, import_path: &str) -> bool {
    let Some((dir, _)) = file_path.rsplit_once('/') else {
        return !import_path.contains('/');
    };
    let dir_segs: Vec<&str> = dir.split('/').filter(|s| !s.is_empty()).collect();
    let imp_segs: Vec<&str> = import_path.split('/').filter(|s| !s.is_empty()).collect();
    if dir_segs.is_empty() || dir_segs.len() > imp_segs.len() {
        return false;
    }
    dir_segs
        .iter()
        .rev()
        .zip(imp_segs.iter().rev())
        .all(|(d, i)| d == i)
}

/// `go_import::import_identifier`: the alias, else the package's name (`.../jwt/v5` is `jwt`).
pub fn go_import_identifier(name: &str) -> Option<String> {
    let name = name.trim();
    if let Some((_, alias)) = name.rsplit_once(" as ") {
        let alias = alias.trim();
        return (!alias.is_empty()).then(|| alias.to_string());
    }
    // `package_identifier_from_path`.
    let path = name.trim_end_matches('/');
    if path.is_empty() {
        return None;
    }
    let is_version = |seg: &str| {
        seg.strip_prefix('v')
            .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
    };
    let mut segments = path.rsplit('/');
    let last = segments.next().unwrap_or(path);
    let candidate = if is_version(last) {
        segments.next().filter(|s| !s.is_empty()).unwrap_or(last)
    } else {
        last
    };
    let mut chars = candidate.chars();
    let valid = matches!(chars.next(), Some(c) if c == '_' || c.is_alphabetic())
        && chars.all(|c| c == '_' || c.is_alphanumeric());
    valid.then(|| candidate.to_string())
}

// ---- js_specifier.rs ----

const EXTENSIONLESS_SUFFIXES: &[&str] = &[
    ".ts", ".tsx", ".d.ts", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs",
];

fn extension(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.rfind('.').map_or("", |i| &name[i + 1..])
}

pub fn is_js_family_importer(path: &str) -> bool {
    matches!(
        extension(path),
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts" | "vue" | "svelte" | "astro"
    )
}

pub fn is_relative_specifier(specifier: &str) -> bool {
    specifier == "."
        || specifier == ".."
        || specifier.starts_with("./")
        || specifier.starts_with("../")
}

fn join_index(dir: &str, suffix: &str) -> String {
    if dir.is_empty() {
        format!("index{suffix}")
    } else {
        format!("{dir}/index{suffix}")
    }
}

fn normalize_join(base_dir: &str, relative: &str) -> Option<String> {
    let mut segments: Vec<&str> = base_dir.split('/').filter(|s| !s.is_empty()).collect();
    for segment in relative.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            other => segments.push(other),
        }
    }
    Some(segments.join("/"))
}

/// `relative_module_candidates`: the project paths a relative JS/TS import may name, in order.
pub fn relative_module_candidates(importer: &str, specifier: &str) -> Vec<String> {
    if !is_js_family_importer(importer) || !is_relative_specifier(specifier) {
        return Vec::new();
    }
    let specifier = specifier.split(['?', '#']).next().unwrap_or(specifier);
    let base_dir = importer.rfind('/').map_or("", |i| &importer[..i]);
    let Some(target) = normalize_join(base_dir, specifier) else {
        return Vec::new();
    };
    let mut out: Vec<String> = Vec::new();
    let mut push = |path: String| {
        if !path.is_empty() && !out.contains(&path) {
            out.push(path);
        }
    };
    if specifier.ends_with('/') || specifier == "." || specifier == ".." {
        for suffix in EXTENSIONLESS_SUFFIXES {
            push(join_index(&target, suffix));
        }
        return out;
    }
    let ts_importer = matches!(extension(importer), "ts" | "tsx" | "mts" | "cts");
    let file_name = target.rsplit('/').next().unwrap_or(&target);
    let mapped: Option<(&str, &[&str])> = match extension(file_name) {
        "js" => Some(("js", &[".ts", ".tsx", ".d.ts"])),
        "jsx" => Some(("jsx", &[".tsx"])),
        "mjs" => Some(("mjs", &[".mts", ".d.mts"])),
        "cjs" => Some(("cjs", &[".cts", ".d.cts"])),
        _ => None,
    };
    if let Some((ext, sources)) = mapped {
        let stem = &target[..target.len() - ext.len() - 1];
        if !ts_importer {
            push(target.clone());
        }
        for source in sources {
            push(format!("{stem}{source}"));
        }
        push(target.clone());
        return out;
    }
    push(target.clone());
    for suffix in EXTENSIONLESS_SUFFIXES {
        push(format!("{target}{suffix}"));
    }
    for suffix in EXTENSIONLESS_SUFFIXES {
        push(join_index(&target, suffix));
    }
    out
}

// ---- the resolver ----

/// `kind_compatible`, our kinds: a call targets a callable, `extends`/`implements` a trait, an
/// interface, a class or a type alias; the rest is permissive.
fn kind_compatible(uref: &Uref, kind: &str) -> bool {
    match uref.kind.as_str() {
        "call" => matches!(kind, "function" | "method" | "macro" | "component"),
        "extends" | "implements" => matches!(kind, "trait" | "interface" | "class" | "type"),
        _ => true,
    }
}

/// `score_candidate`'s callable kinds.
fn is_callable_kind(kind: &str) -> bool {
    matches!(kind, "function" | "method")
}

/// `bare_name_needs_evidence`.
fn bare_name_needs_evidence(lang: &str) -> bool {
    matches!(lang, "python" | "javascript" | "typescript")
}

fn resolved(uref: &Uref, to: &Node, confidence: f64, by: &str) -> Resolved {
    Resolved {
        uref: uref.clone(),
        to: to.id,
        confidence,
        by: by.to_string(),
    }
}

/// What a file imports, as tokensave's `Use` nodes: the file and the name.
pub struct Import<'a> {
    pub path: &'a str,
    pub name: &'a str,
}

pub struct Resolver<'a> {
    name_cache: HashMap<&'a str, Vec<&'a Node>>,
    qualified: HashMap<&'a str, Vec<&'a Node>>,
    suffix: HashMap<&'a str, Vec<&'a str>>,
    known: HashSet<&'a str>,
    import_index: HashMap<String, HashSet<String>>,
    go_qualifiers: HashMap<String, HashMap<String, String>>,
    by_id: HashMap<i64, &'a Node>,
    /// Ours: the fields by name, for `field::method`.
    fields: HashMap<&'a str, Vec<&'a Node>>,
}

impl<'a> Resolver<'a> {
    /// `from_nodes`: the caches, from the symbols seen, the files and the imports.
    pub fn new(nodes: &'a [Node], files: &HashSet<String>, imports: &[Import]) -> Self {
        let mut name_cache: HashMap<&str, Vec<&Node>> = HashMap::new();
        let mut qualified: HashMap<&str, Vec<&Node>> = HashMap::new();
        let mut suffix: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut by_id: HashMap<i64, &Node> = HashMap::new();
        let mut fields: HashMap<&str, Vec<&Node>> = HashMap::new();
        for node in nodes {
            by_id.insert(node.id, node);
            name_cache.entry(node.name.as_str()).or_default().push(node);
            let qn = node.qname.as_str();
            qualified.entry(qn).or_default().push(node);
            let mut pos = 0;
            while let Some(idx) = qn[pos..].find("::") {
                let s = &qn[pos + idx + 2..];
                if !s.is_empty() {
                    suffix.entry(s).or_default().push(qn);
                }
                pos += idx + 2;
            }
            if node.kind == "field" && !node.ty.is_empty() {
                fields.entry(node.name.as_str()).or_default().push(node);
            }
        }
        for entries in suffix.values_mut() {
            entries.sort_unstable();
            entries.dedup();
        }
        let mut known: HashSet<&str> = HashSet::new();
        known.extend(name_cache.keys().copied());
        known.extend(qualified.keys().copied());
        known.extend(suffix.keys().copied());

        let mut import_index: HashMap<String, HashSet<String>> = HashMap::new();
        let mut go_qualifiers: HashMap<String, HashMap<String, String>> = HashMap::new();
        for imp in imports {
            let imported = imp.name.rsplit("::").next().unwrap_or(imp.name);
            if imported != "*" {
                import_index
                    .entry(imp.path.to_string())
                    .or_default()
                    .insert(imported.to_string());
            }
            if let Some(target) = relative_module_candidates(imp.path, imp.name)
                .into_iter()
                .find(|p| files.contains(p))
            {
                import_index
                    .entry(imp.path.to_string())
                    .or_default()
                    .insert(target);
            }
            if lang_from_path(imp.path) == "go" {
                let path = imp
                    .name
                    .split_once(" as ")
                    .map_or(imp.name, |(p, _)| p)
                    .trim();
                if let Some(q) = go_import_identifier(imp.name) {
                    if q != "_" && q != "." {
                        go_qualifiers
                            .entry(imp.path.to_string())
                            .or_default()
                            .insert(q, path.to_string());
                    }
                }
            }
        }
        Self {
            name_cache,
            qualified,
            suffix,
            known,
            import_index,
            go_qualifiers,
            by_id,
            fields,
        }
    }

    /// The keys the name pre-filter admits (`known_names`), for the test that pins the touched set
    /// to it.
    pub fn known_names(&self) -> &HashSet<&'a str> {
        &self.known
    }

    pub fn node(&self, id: i64) -> Option<&'a Node> {
        self.by_id.get(&id).copied()
    }

    /// `resolve_one`: the strategies, in tokensave's order.
    pub fn resolve_one(&self, uref: &Uref) -> Option<Resolved> {
        // Ours: `field::method`, through the field's type, or nothing (its siblings fall back).
        if uref.via == 2 {
            return self.try_field_typed_match(uref);
        }
        // Strategy 1: qualified name match.
        if uref.name.contains("::") {
            if let Some(r) = self.try_qualified_match(uref) {
                return Some(r);
            }
            let simple = uref.name.rsplit("::").next().unwrap_or(&uref.name);
            return self.try_exact_name_match_simple(uref, simple, false, PATH_TAIL_MATCH);
        }
        // Strategy 1b: dotted receiver call.
        if uref.name.contains('.') {
            if let Some(r) = self.try_go_selector_match(uref) {
                return Some(r);
            }
            let simple = uref.name.rsplit('.').next().unwrap_or(&uref.name);
            if simple != uref.name {
                return self.try_exact_name_match_simple(uref, simple, true, SIMPLE_NAME_MATCH);
            }
            return None;
        }
        // Strategy 2: exact name match.
        self.try_exact_name_match(uref)
    }

    /// `resolve_all`: the pre-filter, the confidence floor, the ties explained, the siblings of a
    /// resolved typed call dropped.
    pub fn resolve_all(&self, refs: &[Uref]) -> Outcome {
        let mut out = Outcome::default();
        let mut failed: Vec<&Uref> = Vec::new();
        for uref in refs {
            let known = self.known.contains(uref.name.as_str())
                || self.known.contains(simple_ref_name(&uref.name));
            if !known {
                failed.push(uref);
                continue;
            }
            match self.resolve_one(uref) {
                Some(r) if r.confidence >= CONFIDENCE_FLOOR => out.resolved.push(r),
                _ => failed.push(uref),
            }
        }
        out.ambiguous = failed
            .iter()
            .filter_map(|u| self.explain_ambiguity(u))
            .collect();
        out.unresolved = failed.len() - out.ambiguous.len();
        suppress_go_selector_bare_siblings(&mut out.resolved);
        suppress_typed_siblings(&mut out.resolved, &mut out.ambiguous);
        out
    }

    fn try_qualified_match(&self, uref: &Uref) -> Option<Resolved> {
        if let Some(c) = self.qualified.get(uref.name.as_str()) {
            if let Some(node) = c.iter().find(|n| kind_compatible(uref, &n.kind)) {
                return Some(resolved(uref, node, 0.95, QUALIFIED_MATCH));
            }
        }
        if let Some(full_names) = self.suffix.get(uref.name.as_str()) {
            for full in full_names {
                if let Some(c) = self.qualified.get(full) {
                    if let Some(node) = c.iter().find(|n| kind_compatible(uref, &n.kind)) {
                        return Some(resolved(uref, node, 0.95, QUALIFIED_MATCH));
                    }
                }
            }
        }
        None
    }

    /// Ours: `field::method`. The field's type is that of the fields of this name in the caller's
    /// file if they agree, else that of every field of this name if they agree; then `Type::method`
    /// as a qualified name.
    fn try_field_typed_match(&self, uref: &Uref) -> Option<Resolved> {
        let (field, method) = uref.name.split_once("::")?;
        let all = self.fields.get(field)?;
        let one_type = |fs: &[&&Node]| -> Option<String> {
            let first = fs.first()?.ty.clone();
            fs.iter().all(|f| f.ty == first).then_some(first)
        };
        let lang = lang_from_path(&uref.path);
        let same_lang: Vec<&&Node> = all
            .iter()
            .filter(|f| lang_from_path(&f.path) == lang)
            .collect();
        let in_file: Vec<&&Node> = same_lang
            .iter()
            .copied()
            .filter(|f| f.path == uref.path)
            .collect();
        let ty = if in_file.is_empty() {
            one_type(&same_lang)?
        } else {
            one_type(&in_file)?
        };
        let typed = Uref {
            name: format!("{ty}::{method}"),
            ..uref.clone()
        };
        let mut r = self.try_qualified_match(&typed)?;
        r.uref = uref.clone();
        r.by = FIELD_TYPED.to_string();
        Some(r)
    }

    fn try_go_selector_match(&self, uref: &Uref) -> Option<Resolved> {
        if lang_from_path(&uref.path) != "go" {
            return None;
        }
        let (qualifier, name) = uref.name.split_once('.')?;
        if name.contains('.') {
            return None;
        }
        let import_path = self.go_qualifiers.get(&uref.path)?.get(qualifier)?;
        let candidates = self.name_cache.get(name)?;
        let matched: Vec<&Node> = candidates
            .iter()
            .copied()
            .filter(|n| kind_compatible(uref, &n.kind))
            .filter(|n| go_file_in_package(&n.path, import_path))
            .collect();
        if matched.len() == 1 {
            return Some(resolved(uref, matched[0], 0.95, "go-selector-import"));
        }
        if matched.len() > 1 {
            let winners = self.find_best_matches(uref, &matched);
            let [best] = winners.as_slice() else {
                return None;
            };
            return Some(resolved(uref, best, 0.9, "go-selector-import"));
        }
        None
    }

    fn try_exact_name_match(&self, uref: &Uref) -> Option<Resolved> {
        if CROSS_FILE_BLOCKLIST.contains(&uref.name.as_str()) {
            return self.same_file_blocklisted(uref, &uref.name);
        }
        let raw = self.name_cache.get(uref.name.as_str())?;
        let filtered: Vec<&Node> = raw
            .iter()
            .copied()
            .filter(|n| kind_compatible(uref, &n.kind))
            .collect();
        if filtered.is_empty() {
            return None;
        }
        if filtered.len() != raw.len() {
            return self.resolve_from_filtered_named(uref, &filtered, "exact-match", false);
        }
        if filtered.len() == 1 {
            let ref_lang = lang_from_path(&uref.path);
            if bare_name_needs_evidence(ref_lang) && !self.is_plausibly_reachable(uref, filtered[0])
            {
                return None;
            }
            return Some(resolved(
                uref,
                filtered[0],
                self.lone_confidence(uref, filtered[0]),
                "exact-match",
            ));
        }
        let winners = self.find_best_matches(uref, &filtered);
        let [best] = winners.as_slice() else {
            return None;
        };
        Some(resolved(uref, best, 0.7, "exact-match-scored"))
    }

    fn try_exact_name_match_simple(
        &self,
        uref: &Uref,
        simple: &str,
        require_reachable: bool,
        tag: &str,
    ) -> Option<Resolved> {
        if CROSS_FILE_BLOCKLIST.contains(&simple) {
            return self.same_file_blocklisted(uref, simple);
        }
        let raw = self.name_cache.get(simple)?;
        let filtered: Vec<&Node> = raw
            .iter()
            .copied()
            .filter(|n| kind_compatible(uref, &n.kind))
            .collect();
        if filtered.is_empty() {
            return None;
        }
        if filtered.len() != raw.len() {
            return self.resolve_from_filtered_named(uref, &filtered, tag, require_reachable);
        }
        if filtered.len() == 1 {
            if require_reachable && !self.is_plausibly_reachable(uref, filtered[0]) {
                return None;
            }
            return Some(resolved(
                uref,
                filtered[0],
                self.lone_confidence(uref, filtered[0]),
                tag,
            ));
        }
        let winners = self.find_best_matches(uref, &filtered);
        let [best] = winners.as_slice() else {
            return None;
        };
        Some(resolved(uref, best, 0.7, &format!("{tag}-scored")))
    }

    /// A blocklisted name binds in its own file only, to one compatible symbol.
    fn same_file_blocklisted(&self, uref: &Uref, name: &str) -> Option<Resolved> {
        let candidates = self.name_cache.get(name)?;
        let same_file: Vec<&Node> = candidates
            .iter()
            .copied()
            .filter(|n| n.path == uref.path)
            .filter(|n| kind_compatible(uref, &n.kind))
            .collect();
        if same_file.len() == 1 {
            return Some(resolved(uref, same_file[0], 0.9, "same-file-blocklist"));
        }
        None
    }

    /// 0.9, or 0.5 across two known languages (under the floor: dropped).
    fn lone_confidence(&self, uref: &Uref, node: &Node) -> f64 {
        let r = lang_from_path(&uref.path);
        let c = lang_from_path(&node.path);
        if r != "unknown" && c != "unknown" && !same_language_family(r, c) {
            0.5
        } else {
            0.9
        }
    }

    fn resolve_from_filtered_named(
        &self,
        uref: &Uref,
        filtered: &[&Node],
        by: &str,
        require_reachable: bool,
    ) -> Option<Resolved> {
        if filtered.len() == 1 {
            if require_reachable && !self.is_plausibly_reachable(uref, filtered[0]) {
                return None;
            }
            return Some(resolved(uref, filtered[0], 0.85, by));
        }
        let winners = self.find_best_matches(uref, filtered);
        let [best] = winners.as_slice() else {
            return None;
        };
        Some(resolved(uref, best, 0.65, &format!("{by}-scored")))
    }

    /// `is_plausibly_reachable`: the same file, the same folder, an imported file, name, holder or
    /// module.
    fn is_plausibly_reachable(&self, uref: &Uref, candidate: &Node) -> bool {
        if candidate.path == uref.path {
            return true;
        }
        let dir_of = |p: &str| p.rfind('/').map(|i| p[..i].to_string());
        if dir_of(&candidate.path) == dir_of(&uref.path) {
            return true;
        }
        let Some(imports) = self.import_index.get(&uref.path) else {
            return false;
        };
        if imports.contains(&candidate.path) {
            return true;
        }
        let imported = |name: &str| {
            imports
                .iter()
                .any(|e| e == name || e.rsplit('.').next() == Some(name))
        };
        if imported(&candidate.name) {
            return true;
        }
        if !candidate.parent.is_empty() && imported(&candidate.parent) {
            return true;
        }
        let module = candidate
            .path
            .rsplit('/')
            .next()
            .and_then(|f| f.split('.').next());
        module.is_some_and(imported)
    }

    /// `score_candidate`.
    pub fn score_candidate(&self, uref: &Uref, node: &Node) -> i64 {
        let ref_lang = lang_from_path(&uref.path);
        let mut score: i64 = 0;
        if node.path == uref.path {
            score += 100;
            let distance = node.line.abs_diff(uref.line);
            let proximity = 20_i64.saturating_sub(i64::from(distance) / 10);
            score += proximity.max(0);
        } else {
            score += path_proximity(&uref.path, &node.path);
        }
        let cand_lang = lang_from_path(&node.path);
        if ref_lang != "unknown" && cand_lang != "unknown" {
            if same_language_family(ref_lang, cand_lang) {
                score += 50;
            } else {
                score -= 80;
            }
        }
        if node.vis {
            score += 10;
        }
        if uref.kind == "call" && is_callable_kind(&node.kind) {
            score += 25;
        }
        if let Some(imports) = self.import_index.get(&uref.path) {
            if imports.contains(&node.name) {
                score += 30;
            }
        }
        score
    }

    /// `find_best_matches`: every candidate tied for the best score, by id.
    fn find_best_matches(&self, uref: &Uref, candidates: &[&'a Node]) -> Vec<&'a Node> {
        let scored: Vec<(i64, &Node)> = candidates
            .iter()
            .map(|n| (self.score_candidate(uref, n), *n))
            .collect();
        let Some(best) = scored.iter().map(|(s, _)| *s).max() else {
            return Vec::new();
        };
        let mut winners: Vec<&Node> = scored
            .into_iter()
            .filter(|(s, _)| *s == best)
            .map(|(_, n)| n)
            .collect();
        winners.sort_by_key(|n| n.id);
        winners
    }

    /// `explain_ambiguity`: the candidates of a call left unresolved by a tie.
    fn explain_ambiguity(&self, uref: &Uref) -> Option<Ambiguous> {
        if uref.kind != "call" || uref.via == 2 {
            return None;
        }
        let simple = simple_ref_name(&uref.name);
        let raw = self.name_cache.get(simple)?;
        let candidates: Vec<&Node> = raw
            .iter()
            .copied()
            .filter(|n| kind_compatible(uref, &n.kind))
            .collect();
        let winners = self.find_best_matches(uref, &candidates);
        if winners.len() < 2 {
            return None;
        }
        Some(Ambiguous {
            uref: uref.clone(),
            candidates: winners.into_iter().map(|n| n.id).collect(),
        })
    }
}

/// A call site: caller, file, line, column, and the method's bare name.
type Site<'r> = (Option<i64>, &'r str, u32, u32, &'r str);

fn site(u: &Uref) -> Site<'_> {
    (
        u.from,
        u.path.as_str(),
        u.line,
        u.col,
        simple_ref_name(&u.name),
    )
}

/// `suppress_go_selector_bare_siblings`.
fn suppress_go_selector_bare_siblings(resolved: &mut Vec<Resolved>) {
    let sites: HashSet<(Option<i64>, String, u32, u32, String)> = resolved
        .iter()
        .filter(|r| r.by == "go-selector-import")
        .filter_map(|r| {
            let bare = r.uref.name.rsplit('.').next()?;
            Some((
                r.uref.from,
                r.uref.path.clone(),
                r.uref.line,
                r.uref.col,
                bare.to_string(),
            ))
        })
        .collect();
    if sites.is_empty() {
        return;
    }
    resolved.retain(|r| {
        r.uref.name.contains('.')
            || !sites.contains(&(
                r.uref.from,
                r.uref.path.clone(),
                r.uref.line,
                r.uref.col,
                r.uref.name.clone(),
            ))
    });
}

/// `suppress_gdscript_typed_siblings` and `finalize_ambiguous`, for every language: where a typed
/// call (`Type::method` from the extractor, or `field::method`) resolved through its type, the
/// name-based siblings at the same call site and the tie they made are dropped.
fn suppress_typed_siblings(resolved: &mut Vec<Resolved>, ambiguous: &mut Vec<Ambiguous>) {
    let typed = |r: &Resolved| {
        r.uref.via > 0 && r.uref.kind == "call" && (r.by == QUALIFIED_MATCH || r.by == FIELD_TYPED)
    };
    let sites: HashSet<(Option<i64>, String, u32, u32, String)> = resolved
        .iter()
        .filter(|r| typed(r))
        .map(|r| {
            let s = site(&r.uref);
            (s.0, s.1.to_string(), s.2, s.3, s.4.to_string())
        })
        .collect();
    if sites.is_empty() {
        return;
    }
    let key = |u: &Uref| {
        let s = site(u);
        (s.0, s.1.to_string(), s.2, s.3, s.4.to_string())
    };
    resolved.retain(|r| typed(r) || r.uref.kind != "call" || !sites.contains(&key(&r.uref)));
    ambiguous.retain(|a| !sites.contains(&key(&a.uref)));
}

/// `touched.rs::index_keys`: every key a symbol is reachable under in the name index.
pub fn index_keys(name: &str, qname: &str, out: &mut HashSet<String>) {
    out.insert(name.to_string());
    out.insert(qname.to_string());
    let mut pos = 0;
    while let Some(idx) = qname[pos..].find("::") {
        let s = &qname[pos + idx + 2..];
        if !s.is_empty() {
            out.insert(s.to_string());
        }
        pos += idx + 2;
    }
}

/// `touched.rs::TouchedSet`: what a reading could have changed the resolution of.
#[derive(Debug, Default)]
pub struct Touched {
    pub files: HashSet<String>,
    pub names: HashSet<String>,
}

impl Touched {
    pub fn touch_symbol(&mut self, name: &str, qname: &str) {
        index_keys(name, qname, &mut self.names);
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.names.is_empty()
    }

    /// `needs_resolve`: a reference of a file read again, or whose name or simple name gained or lost
    /// a symbol, or (ours) whose field, for `field::method`, did. (tokensave's relative imports are
    /// left out: Escouade does not resolve the imports.) `links::select` asks SQLite the same.
    pub fn needs_resolve(&self, path: &str, name: &str, via: u8) -> bool {
        self.files.contains(path)
            || self.names.contains(name)
            || self.names.contains(simple_ref_name(name))
            || (via == 2
                && name
                    .split("::")
                    .next()
                    .is_some_and(|f| self.names.contains(f)))
    }
}
