//! Ported from tokensave `src/context/ranking.rs` and `src/tokensave/util.rs` (`kind_tier`,
//! `kind_rank_bonus`) (github.com/aovestdipaperino/tokensave, commit 580936b, MIT, © 2025 Enzo
//! Lombardi; see THIRD-PARTY-NOTICES.md): the multiplicative re-ranking of the candidates of a
//! search, same factors. tokensave's `NodeKind` is our kind, a word; each of ours is read as the
//! tokensave kind it is (`kind_of`).

use super::api::Scored;

/// The tokensave kind of one of ours. A Markdown heading is a `Module` there (its Markdown
/// extractor makes them so); a Svelte component, which tokensave has not (it reads the scripts
/// only), is read as a `Class`; a Rust `static` is our `const`.
fn kind_of(kind: &str) -> &str {
    match kind {
        "heading" => "module",
        "component" => "class",
        "type" => "type_alias",
        other => other,
    }
}

/// Boost factor based on node kind.
pub fn kind_boost(kind: &str) -> f64 {
    match kind_of(kind) {
        "function" | "method" | "route" | "constructor" => 2.0,
        "arrow_function" => 1.8,
        "struct" | "class" | "enum" | "trait" | "interface" | "record" | "union" => 1.5,
        "module" | "impl" | "namespace" => 1.2,
        "field" | "property" => 0.5,
        "enum_variant" => 0.3,
        "use" | "export" | "include" => 0.2,
        _ => 1.0,
    }
}

/// Boost factor based on visibility (`pub`, `crate`, `super`, `private`).
pub fn visibility_boost(visibility: &str) -> f64 {
    match visibility {
        "pub" => 1.5,
        "crate" | "super" => 1.2,
        _ => 0.8,
    }
}

/// Returns `true` for paths under test or fixture trees — the same
/// classification `path_boost` uses for down-ranking.
pub fn is_test_path(file_path: &str) -> bool {
    path_boost(file_path) < 1.0
}

/// Boost factor based on file path.
pub fn path_boost(file_path: &str) -> f64 {
    if file_path.contains("tests/fixtures/")
        || file_path.contains("test/fixtures/")
        || file_path.contains("testdata/")
        || file_path.contains("__fixtures__/")
    {
        return 0.1;
    }
    if file_path.starts_with("tests/")
        || file_path.starts_with("test/")
        || file_path.contains("_test.")
        || file_path.contains(".test.")
        || file_path.contains("_spec.")
        || file_path.contains(".spec.")
    {
        return 0.4;
    }
    1.0
}

/// Down-weights test symbols that `path_boost` cannot see: an inline
/// `#[cfg(test)] mod tests` in `src/*.rs` carries a production file path, so
/// its `test_*` functions rank as first-party code and can outrank the very
/// symbol they exercise. Detect them by symbol shape — a `tests`/`test`
/// module segment in the qualified name, or a `test_` name prefix — and
/// apply the same 0.4 factor as test files.
pub fn test_symbol_penalty(name: &str, qualified_name: &str) -> f64 {
    if name.starts_with("test_")
        || qualified_name.contains("::tests::")
        || qualified_name.contains("::test::")
    {
        return 0.4;
    }
    1.0
}

/// Path segments of generated / vendored / dependency trees.
const VENDOR_PATH_SEGMENTS: &[&str] = &[
    "node_modules",
    "dist",
    "build",
    "target",
    "venv",
    ".venv",
    "site-packages",
    "vendor",
    "__pycache__",
    ".next",
    "out",
];

/// Path segments of non-production trees: examples, samples, benchmarks, demos.
const NON_PROD_PATH_SEGMENTS: &[&str] = &[
    "examples",
    "example",
    "samples",
    "sample",
    "benchmarks",
    "benchmark",
    "demos",
    "demo",
];

/// Path segments of typical application source directories.
const APP_PATH_SEGMENTS: &[&str] = &["src", "app", "lib"];

fn has_path_segment(normalized: &str, segment: &str) -> bool {
    normalized.split('/').any(|c| c == segment)
}

/// File extensions of prose documentation formats.
const DOC_FILE_EXTENSIONS: &[&str] = &["md", "markdown", "mdx", "rst", "adoc"];

fn is_doc_file(normalized: &str) -> bool {
    normalized.rsplit('.').next().is_some_and(|ext| {
        DOC_FILE_EXTENSIONS
            .iter()
            .any(|d| ext.eq_ignore_ascii_case(d))
    })
}

/// Path-based ranking multiplier: 0.4 under a vendor / generated tree, 0.3 for a documentation
/// file, 0.6 under a non-production tree, 1.25 under `src/`, `app/` or `lib/`, else 1.0.
pub fn path_rank_multiplier(file_path: &str) -> f64 {
    let normalized = file_path.replace('\\', "/");
    if VENDOR_PATH_SEGMENTS
        .iter()
        .any(|seg| has_path_segment(&normalized, seg))
    {
        return 0.4;
    }
    if is_doc_file(&normalized) {
        return 0.3;
    }
    if NON_PROD_PATH_SEGMENTS
        .iter()
        .any(|seg| has_path_segment(&normalized, seg))
    {
        return 0.6;
    }
    if APP_PATH_SEGMENTS
        .iter()
        .any(|seg| has_path_segment(&normalized, seg))
    {
        return 1.25;
    }
    1.0
}

fn by_score(candidates: &mut [Scored]) {
    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Applies a log-scale connectivity boost based on incoming call counts (by symbol id).
pub fn apply_connectivity_boost(
    candidates: &mut [Scored],
    call_counts: &std::collections::HashMap<i64, u64>,
) {
    for candidate in candidates.iter_mut() {
        let count = call_counts.get(&candidate.node.id).copied().unwrap_or(0);
        // log2(count + 1) scaled to 1.0–2.0 range, capped at 4.0 bits
        let boost = 1.0 + (count as f64 + 1.0).log2().min(4.0) / 4.0;
        candidate.score *= boost;
    }
    by_score(candidates);
}

/// True for a VHDL or Verilog source file.
fn is_hdl_path(path: &str) -> bool {
    let ext = path.rsplit('.').next().unwrap_or("");
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "vhd" | "vhdl" | "v" | "vh" | "sv" | "svh"
    )
}

/// Boost for a node's kind, with one language-aware exception (an HDL module).
pub fn node_kind_boost(kind: &str, path: &str) -> f64 {
    if kind_of(kind) == "module" && is_hdl_path(path) {
        return 2.0;
    }
    kind_boost(kind)
}

/// Re-ranks search result candidates using structural signals.
pub fn rerank_candidates(candidates: &mut [Scored]) {
    for candidate in candidates.iter_mut() {
        let n = &candidate.node;
        let boost = node_kind_boost(&n.kind, &n.path)
            * visibility_boost(&n.vis)
            * path_boost(&n.path)
            * path_rank_multiplier(&n.path)
            * test_symbol_penalty(&n.name, &n.qualified());
        candidate.score *= boost;
    }
    by_score(candidates);
}

/// Applies an additional ranking signal when a task explicitly asks about runtime behavior.
pub fn apply_executable_intent_boost(candidates: &mut [Scored], query: &str) {
    const BEHAVIOR_TERMS: &[&str] = &[
        "branch",
        "cache",
        "converge",
        "convergence",
        "dispatch",
        "failure",
        "fallback",
        "loop",
        "rebuild",
        "retry",
        "retries",
    ];
    let query = query.to_lowercase();
    if !BEHAVIOR_TERMS.iter().any(|term| query.contains(term)) {
        return;
    }
    for candidate in candidates.iter_mut() {
        if is_executable_kind(&candidate.node.kind) {
            candidate.score *= 1.5;
        } else if matches!(kind_of(&candidate.node.kind), "use" | "export" | "include") {
            candidate.score *= 0.1;
        }
    }
    by_score(candidates);
}

/// The executable kinds (tokensave `is_executable_kind`).
pub fn is_executable_kind(kind: &str) -> bool {
    matches!(
        kind_of(kind),
        "function" | "method" | "constructor" | "arrow_function"
    )
}

/// The kinds that own an API (tokensave `is_api_owner_kind`).
pub fn is_api_owner_kind(kind: &str) -> bool {
    matches!(
        kind_of(kind),
        "struct" | "class" | "enum" | "trait" | "interface" | "impl" | "record"
    )
}

/// Coarse ranking tier used as the primary sort key of a search. Lower numbers sort first.
pub fn kind_tier(kind: &str) -> u8 {
    match kind_of(kind) {
        "function" | "route" | "method" | "constructor" | "arrow_function" | "struct" | "enum"
        | "trait" | "class" | "interface" | "record" | "type_alias" | "union" => 0,
        "impl" => 1,
        "const" | "static" | "macro" | "enum_variant" | "field" | "property" | "export" => 2,
        "use" | "include" => 4,
        // module, namespace, file and what we do not know.
        _ => 3,
    }
}

/// Search-result rank bonus applied per node kind, so definitions outrank references.
pub fn kind_rank_bonus(kind: &str) -> f64 {
    match kind_of(kind) {
        "route" => 2.0,
        "function" | "method" | "constructor" | "arrow_function" => 3.0,
        "struct" | "enum" | "trait" | "class" | "interface" | "record" | "type_alias" | "union" => {
            2.5
        }
        "impl" => 2.0,
        "const" | "static" | "macro" | "enum_variant" => 1.0,
        "field" | "property" | "export" => 0.5,
        "use" | "include" => -3.0,
        "module" | "namespace" => -1.5,
        _ => 0.0,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::codeindex::api::Node;

    fn make_result(kind: &str, vis: &str, path: &str, score: f64) -> Scored {
        Scored {
            node: Node {
                id: 1,
                name: "test_sym".into(),
                kind: kind.into(),
                parent: String::new(),
                path: path.into(),
                line: 1,
                end_line: 5,
                sig: String::new(),
                doc: String::new(),
                vis: vis.into(),
            },
            score,
        }
    }

    #[test]
    fn test_function_outranks_field_same_fts_score() {
        let mut candidates = vec![
            make_result("field", "pub", "src/lib.rs", 10.0),
            make_result("function", "pub", "src/lib.rs", 10.0),
        ];
        rerank_candidates(&mut candidates);
        assert_eq!(candidates[0].node.kind, "function");
    }

    #[test]
    fn behavioral_query_prefers_executable_code_over_stronger_import_match() {
        let mut candidates = vec![
            make_result("use", "pub", "src/solver.rs", 10.0),
            make_result("function", "private", "src/solver.rs", 1.0),
        ];
        rerank_candidates(&mut candidates);
        apply_executable_intent_boost(&mut candidates, "retry cache convergence loop");
        assert_eq!(candidates[0].node.kind, "function");
    }

    #[test]
    fn test_public_outranks_private() {
        let mut candidates = vec![
            make_result("function", "private", "src/lib.rs", 10.0),
            make_result("function", "pub", "src/lib.rs", 10.0),
        ];
        rerank_candidates(&mut candidates);
        assert_eq!(candidates[0].node.vis, "pub");
    }

    #[test]
    fn test_fixtures_ranked_below_source() {
        let mut candidates = vec![
            make_result("enum_variant", "pub", "tests/fixtures/sample.m", 10.0),
            make_result("function", "pub", "src/logging.rs", 5.0),
        ];
        rerank_candidates(&mut candidates);
        assert_eq!(candidates[0].node.path, "src/logging.rs");
    }

    #[test]
    fn test_test_files_penalized_vs_source() {
        let mut candidates = vec![
            make_result("function", "pub", "tests/sync_test.rs", 10.0),
            make_result("function", "pub", "src/sync.rs", 10.0),
        ];
        rerank_candidates(&mut candidates);
        assert_eq!(candidates[0].node.path, "src/sync.rs");
    }

    #[test]
    fn test_rerank_preserves_order_when_boosts_equal() {
        let mut candidates = vec![
            make_result("function", "pub", "src/a.rs", 10.0),
            make_result("function", "pub", "src/b.rs", 5.0),
        ];
        rerank_candidates(&mut candidates);
        assert_eq!(candidates[0].node.path, "src/a.rs");
        assert_eq!(candidates[1].node.path, "src/b.rs");
    }

    #[test]
    fn test_enum_variant_low_boost() {
        assert!(kind_boost("enum_variant") < 1.0);
        assert!(kind_boost("function") > 1.0);
    }

    #[test]
    fn hdl_design_units_rank_as_functions_not_modules() {
        assert_eq!(node_kind_boost("module", "rtl/top.vhd"), 2.0);
        assert_eq!(node_kind_boost("module", "rtl/top.sv"), 2.0);
        assert_eq!(
            node_kind_boost("module", "src/lib.rs"),
            kind_boost("module")
        );
        assert!(node_kind_boost("module", "src/lib.rs") < 2.0);
    }

    #[test]
    fn test_kind_boost_values() {
        assert_eq!(kind_boost("function"), 2.0);
        assert_eq!(kind_boost("method"), 2.0);
        assert_eq!(kind_boost("struct"), 1.5);
        assert_eq!(kind_boost("enum_variant"), 0.3);
        assert_eq!(kind_boost("use"), 0.2);
        // Ours, read as tokensave's kinds.
        assert_eq!(kind_boost("heading"), kind_boost("module"));
        assert_eq!(kind_boost("component"), kind_boost("class"));
        assert_eq!(kind_boost("type"), 1.0);
    }

    #[test]
    fn test_visibility_boost_values() {
        assert_eq!(visibility_boost("pub"), 1.5);
        assert_eq!(visibility_boost("private"), 0.8);
        assert_eq!(visibility_boost("crate"), 1.2);
    }

    #[test]
    fn test_path_boost_values() {
        assert_eq!(path_boost("src/lib.rs"), 1.0);
        assert_eq!(path_boost("tests/fixtures/sample.m"), 0.1);
        assert_eq!(path_boost("tests/sync_test.rs"), 0.4);
        assert_eq!(path_boost("test/fixtures/foo.js"), 0.1);
        assert_eq!(path_boost("src/components/Button.test.tsx"), 0.4);
        assert!(is_test_path("tests/sync_test.rs"));
    }

    #[test]
    fn test_symbol_penalty_inline_tests() {
        assert_eq!(
            test_symbol_penalty(
                "test_stem_variants_tion_suffix",
                "src/context/builder.rs::tests::test_stem_variants_tion_suffix"
            ),
            0.4
        );
        assert_eq!(
            test_symbol_penalty("helper", "src/context/builder.rs::tests::helper"),
            0.4
        );
        assert_eq!(
            test_symbol_penalty(
                "generate_stem_variants",
                "src/context/builder.rs::generate_stem_variants"
            ),
            1.0
        );
        assert_eq!(
            test_symbol_penalty("attest", "src/attestation.rs::attest"),
            1.0
        );
    }

    #[test]
    fn test_path_rank_multiplier_vendor_below_one() {
        assert!(path_rank_multiplier("node_modules/foo/index.js") < 1.0);
        assert!(path_rank_multiplier("frontend/node_modules/foo/index.js") < 1.0);
        assert!(path_rank_multiplier("dist/bundle.js") < 1.0);
        assert!(path_rank_multiplier("target/debug/build/x.rs") < 1.0);
        assert!(path_rank_multiplier(".venv/lib/python3.11/site-packages/x.py") < 1.0);
        assert!(path_rank_multiplier("vendor/github.com/foo/bar.go") < 1.0);
        assert!(path_rank_multiplier("project/__pycache__/mod.pyc") < 1.0);
        assert!(path_rank_multiplier("web/.next/server/page.js") < 1.0);
        assert!(path_rank_multiplier("out/main.js") < 1.0);
    }

    #[test]
    fn test_path_rank_multiplier_app_above_neutral() {
        assert!(path_rank_multiplier("src/lib.rs") > 1.0);
        assert!(path_rank_multiplier("app/models/user.rb") > 1.0);
        assert!(path_rank_multiplier("lib/widget.dart") > 1.0);
        assert!(path_rank_multiplier("crate/src/foo.rs") > 1.0);
    }

    #[test]
    fn test_path_rank_multiplier_neutral_is_baseline() {
        assert_eq!(path_rank_multiplier("foo/bar.rs"), 1.0);
    }

    #[test]
    fn test_path_rank_multiplier_doc_factor() {
        for path in [
            "README.md",
            "docs/guide.md",
            "CHANGELOG.markdown",
            "book/src/intro.rst",
            "guide.adoc",
            "blog/post.MDX",
        ] {
            assert_eq!(path_rank_multiplier(path), 0.3, "{path}");
        }
        assert_eq!(path_rank_multiplier("node_modules/foo/README.md"), 0.4);
        assert_eq!(path_rank_multiplier("notes.txt"), 1.0);
        assert_eq!(path_rank_multiplier("mdbook/render.rs"), 1.0);
    }

    #[test]
    fn test_code_outranks_doc_heading_in_rerank() {
        let mut doc = make_result("heading", "pub", "docs/guide.md", 10.0);
        doc.node.name = "Configuration".to_string();
        let code = make_result("function", "private", "src/config.rs", 10.0);
        let mut candidates = vec![doc, code];
        rerank_candidates(&mut candidates);
        assert_eq!(candidates[0].node.path, "src/config.rs");
    }

    #[test]
    fn test_path_rank_multiplier_vendor_wins_over_app() {
        assert!(path_rank_multiplier("node_modules/pkg/src/index.js") < 1.0);
    }

    #[test]
    fn test_path_rank_multiplier_nonprod_below_one() {
        for p in [
            "examples/foo.rs",
            "example/foo.rs",
            "samples/foo.rs",
            "sample/foo.rs",
            "benchmarks/bench.rs",
            "benchmark/bench.rs",
            "demos/app.rs",
            "demo/app.rs",
            "crate/examples/usage.rs",
        ] {
            assert!(path_rank_multiplier(p) < 1.0, "{p}");
        }
    }

    #[test]
    fn test_path_rank_multiplier_nonprod_above_vendor() {
        assert!(
            path_rank_multiplier("examples/foo.rs") > path_rank_multiplier("node_modules/foo.js")
        );
    }

    #[test]
    fn test_path_rank_multiplier_vendor_wins_over_nonprod() {
        assert_eq!(
            path_rank_multiplier("target/examples/foo.rs"),
            path_rank_multiplier("target/debug/foo.rs")
        );
        assert!(
            path_rank_multiplier("target/examples/foo.rs")
                < path_rank_multiplier("examples/foo.rs")
        );
    }

    #[test]
    fn test_path_rank_multiplier_nonprod_substring_not_matched() {
        assert_eq!(path_rank_multiplier("demolition/handler.rs"), 1.0);
        assert_eq!(path_rank_multiplier("exampled/foo.rs"), 1.0);
    }

    #[test]
    fn test_src_outranks_examples_in_rerank() {
        let mut candidates = vec![
            make_result("function", "pub", "examples/handler.rs", 10.0),
            make_result("function", "pub", "src/handler.rs", 10.0),
        ];
        rerank_candidates(&mut candidates);
        assert_eq!(candidates[0].node.path, "src/handler.rs");
    }

    #[test]
    fn test_path_rank_multiplier_normalizes_backslashes() {
        assert!(path_rank_multiplier("web\\node_modules\\foo.js") < 1.0);
        assert!(path_rank_multiplier("crate\\src\\foo.rs") > 1.0);
    }

    #[test]
    fn test_path_rank_multiplier_substring_not_matched() {
        assert_eq!(path_rank_multiplier("outbound/handler.rs"), 1.0);
        assert_eq!(path_rank_multiplier("library/foo.rs"), 1.0);
    }

    #[test]
    fn test_app_source_outranks_vendor_in_rerank() {
        let mut candidates = vec![
            make_result("function", "pub", "node_modules/pkg/index.js", 10.0),
            make_result("function", "pub", "src/handler.rs", 10.0),
        ];
        rerank_candidates(&mut candidates);
        assert_eq!(candidates[0].node.path, "src/handler.rs");
    }

    #[test]
    fn test_connectivity_boost_prefers_high_fanin() {
        let mut a = make_result("function", "pub", "src/a.rs", 10.0);
        a.node.id = 1;
        let mut b = make_result("function", "pub", "src/b.rs", 10.0);
        b.node.id = 2;
        let mut candidates = vec![a, b];
        rerank_candidates(&mut candidates);
        let base_score = candidates[0].score;
        assert_eq!(candidates[1].score, base_score, "same base score");
        let mut counts = std::collections::HashMap::new();
        counts.insert(1i64, 15u64);
        apply_connectivity_boost(&mut candidates, &counts);
        assert_eq!(candidates[0].node.path, "src/a.rs");
        assert!(candidates[0].score > candidates[1].score);
    }

    #[test]
    fn definitions_tier_ahead_of_references() {
        assert_eq!(kind_tier("function"), 0);
        assert_eq!(kind_tier("component"), 0);
        assert_eq!(kind_tier("impl"), 1);
        assert_eq!(kind_tier("field"), 2);
        assert_eq!(kind_tier("heading"), 3);
        assert_eq!(kind_rank_bonus("function"), 3.0);
        assert_eq!(kind_rank_bonus("heading"), -1.5);
        assert_eq!(kind_rank_bonus("field"), 0.5);
    }
}
