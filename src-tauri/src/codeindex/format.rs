//! Ported from tokensave `src/context/formatter.rs` (`format_context_as_markdown`,
//! `compact_signature`, `fence_language`), the text formats of `src/mcp/tools/handlers/graph.rs`
//! (`handle_search`, `handle_literal_search`) and `src/mcp/tools/handlers/info.rs`
//! (`body_result`, `candidates_result`), and `truncate_response` / `truncate_response_keep_tail`
//! of `src/mcp/tools/handlers/mod.rs` (github.com/aovestdipaperino/tokensave, commit 580936b, MIT,
//! © 2025 Enzo Lombardi; see THIRD-PARTY-NOTICES.md): what the index's tools send, as text.
//!
//! What differs, for our architecture: the sentences are in the user's language (French or
//! English), the headings and the keys stay tokensave's; the tools are named `index_*`; the ids
//! are numbers; the metrics line is `index_metrics`.

use super::answers::{member_example, Body, Found, Literal};
use super::api::{Context, Hit};
use super::ranking::is_test_path;
use crate::i18n::Lang;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

/// Longest signature rendered inline for an entry point, in bytes.
pub const MAX_SIGNATURE_LEN: usize = 200;

/// The most characters of an answer (tokensave `MAX_RESPONSE_CHARS`).
pub const MAX_RESPONSE_CHARS: usize = 15_000;

/// Renders a signature as one bounded line.
pub fn compact_signature(sig: &str) -> String {
    let first = sig.lines().next().unwrap_or("").trim_end();
    let truncated_lines = sig.lines().nth(1).is_some();
    match first.char_indices().nth(MAX_SIGNATURE_LEN) {
        Some((idx, _)) => format!("{} …", first[..idx].trim_end()),
        None if truncated_lines => format!("{first} …"),
        None => first.to_string(),
    }
}

/// Markdown fence language for a source file path, derived from its extension.
pub fn fence_language(file_path: &str) -> &'static str {
    let ext = file_path
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "rs" => "rust",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "tsx",
        "js" | "mjs" | "cjs" => "javascript",
        "jsx" => "jsx",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "rb" | "rake" => "ruby",
        "erb" => "erb",
        "slim" => "slim",
        "php" => "php",
        "cs" => "csharp",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "swift" => "swift",
        "dart" => "dart",
        "scala" => "scala",
        "sh" | "bash" | "zsh" => "bash",
        "sql" => "sql",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        "html" => "html",
        "css" => "css",
        "lua" => "lua",
        "ex" | "exs" => "elixir",
        "hs" => "haskell",
        "zig" => "zig",
        "vue" => "vue",
        "svelte" => "svelte",
        _ => "",
    }
}

/// tokensave `format_context_as_markdown`, then `seen_node_ids` (`handle_context`).
pub fn context_markdown(lang: Lang, query: &str, context: &Context) -> String {
    let mut out = String::new();
    out.push_str("## Code Context\n");
    let _ = write!(out, "**Query:** {query}\n\n");

    out.push_str("### Entry Points\n");
    if context.entry_points.is_empty() {
        out.push_str(&tr_in!(
            lang,
            "_Aucun point d’entrée trouvé._\n\n",
            "_No entry points found._\n\n"
        ));
    } else {
        for node in &context.entry_points {
            let _ = writeln!(
                out,
                "- **{}** ({}) - {}:{}",
                node.name, node.kind, node.path, node.line
            );
            if !node.sig.is_empty() {
                let _ = writeln!(out, "  `{}`", compact_signature(&node.sig));
            }
            if let Some(first) = node.doc.lines().find(|line| !line.trim().is_empty()) {
                let _ = writeln!(out, "  {}", first.trim());
            }
        }
        out.push('\n');
    }

    // Related Symbols grouped by file: the subgraph, its roots first. Test/fixture symbols are
    // collapsed to a one-line count.
    out.push_str("### Related Symbols\n");
    let subgraph: Vec<&Hit> = context
        .entry_points
        .iter()
        .chain(&context.related)
        .collect();
    if subgraph.is_empty() {
        out.push_str(&tr_in!(
            lang,
            "_Aucun symbole lié._\n\n",
            "_No related symbols._\n\n"
        ));
    } else {
        let mut by_file: HashMap<&str, Vec<(&str, u32)>> = HashMap::new();
        let mut test_symbols = 0usize;
        let mut test_files: HashSet<&str> = HashSet::new();
        for node in &subgraph {
            if is_test_path(&node.path) {
                test_symbols += 1;
                test_files.insert(&node.path);
                continue;
            }
            by_file
                .entry(&node.path)
                .or_default()
                .push((&node.name, node.line));
        }
        let mut files: Vec<&&str> = by_file.keys().collect();
        files.sort();
        for file in files {
            let formatted: Vec<String> = by_file[*file]
                .iter()
                .map(|(name, line)| format!("{name}:{line}"))
                .collect();
            let _ = writeln!(out, "- {}: {}", file, formatted.join(", "));
        }
        if test_symbols > 0 {
            out.push_str(&tr_in!(
                lang,
                "- fichiers de test : {test_symbols} symboles dans {files} fichiers (index_callers sur un point d’entrée pour le détail)\n",
                "- test/fixture files: {test_symbols} symbols across {files} files (index_callers on an entry point for details)\n",
                test_symbols = test_symbols,
                files = test_files.len()
            ));
        }
        out.push('\n');
    }

    out.push_str("### Code\n");
    if context.snippets.is_empty() {
        out.push_str(&tr_in!(
            lang,
            "_Aucun bloc de code extrait._\n",
            "_No code blocks extracted._\n"
        ));
    } else {
        for block in &context.snippets {
            let label = context
                .entry_points
                .iter()
                .find(|n| n.id == block.id)
                .map_or_else(|| block.id.to_string(), |n| n.name.clone());
            let _ = writeln!(out, "#### {} ({}:{})", label, block.path, block.line);
            let _ = writeln!(out, "```{}", fence_language(&block.path));
            out.push_str(&block.text);
            if !block.text.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n\n");
        }
    }

    let diag = &context.retrieval;
    if !diag.terms.is_empty() || diag.best_score.is_some() || context.entry_points.is_empty() {
        out.push_str("### Retrieval\n");
        match (diag.quality.as_deref(), diag.best_score) {
            (Some(quality), Some(score)) => {
                let score = if score < 0.01 {
                    "<0.01".to_string()
                } else {
                    format!("{score:.2}")
                };
                let _ = writeln!(out, "- match: {quality} (best score {score})");
            }
            _ if diag.terms.is_empty() => out.push_str(&tr_in!(
                lang,
                "- match: none (aucun terme à chercher n’a été tiré de la tâche — reformule avec des identifiants concrets ou ajoute des `keywords`)\n",
                "- match: none (no searchable terms extracted — rephrase with concrete identifiers or add `keywords`)\n"
            )),
            _ => out.push_str("- match: none (no candidates)\n"),
        }
        if !diag.terms.is_empty() {
            const MAX_MISSES_SHOWN: usize = 6;
            let hits: Vec<String> = diag
                .terms
                .iter()
                .filter(|t| t.hits > 0)
                .map(|t| format!("{}({})", t.term, t.hits))
                .collect();
            if !hits.is_empty() {
                let _ = writeln!(out, "- terms: {}", hits.join(", "));
            }
            let misses: Vec<&str> = diag
                .terms
                .iter()
                .filter(|t| t.hits == 0)
                .map(|t| t.term.as_str())
                .collect();
            if !misses.is_empty() {
                let shown = misses.len().min(MAX_MISSES_SHOWN);
                let overflow = misses.len() - shown;
                let mut listed = misses[..shown].join(", ");
                if overflow > 0 {
                    let _ = write!(listed, ", +{overflow} more");
                }
                out.push_str(&tr_in!(
                    lang,
                    "- ⚠ no hits ({n}) : {listed} — ajoute des synonymes avec les mots du code dans `keywords`, ou essaie index_search\n",
                    "- ⚠ no hits ({n}): {listed} — add `keywords` synonyms in the words of the code or try index_search\n",
                    n = misses.len(),
                    listed = listed
                ));
            }
        }
        out.push('\n');
    }

    if !context.entry_points.is_empty() {
        let ids: Vec<i64> = context.entry_points.iter().map(|n| n.id).collect();
        let _ = write!(
            out,
            "\nseen_node_ids: {}\n",
            serde_json::to_string(&ids).unwrap_or_default()
        );
    }
    truncate_response_keep_tail(&out, "### Retrieval")
}

/// The text format of `index_search` (tokensave `handle_search`, `format: text`).
pub fn search_text(lang: Lang, found: &Found, ids: bool) -> String {
    let n = found.hits.len();
    let mut text = if found.truncated {
        tr_in!(
            lang,
            "count: {n} (tronqué : il y a d’autres résultats ; monte `limit` pour les voir)\n\n",
            "count: {n} (truncated: more matches exist; raise `limit` to see them)\n\n",
            n = n
        )
    } else {
        format!("count: {n}\n\n")
    };
    for h in &found.hits {
        let id = if ids {
            format!(" [id: {}]", h.id)
        } else {
            String::new()
        };
        if h.sig.is_empty() {
            let _ = writeln!(
                text,
                "{}:{}-{}: {} ({}){id}",
                h.path, h.line, h.end_line, h.name, h.kind
            );
        } else {
            let _ = writeln!(
                text,
                "{}:{}-{}: {} ({}){id} — {}",
                h.path,
                h.line,
                h.end_line,
                h.name,
                h.kind,
                compact_signature(&h.sig)
            );
        }
    }
    truncate_response(&text)
}

/// The text format of a literal `index_search` (tokensave `handle_literal_search`).
pub fn literal_text(lang: Lang, l: &Literal, ids: bool) -> String {
    let count = l.matches.len();
    let total = l.total;
    let mut text = if l.timed_out {
        tr_in!(
            lang,
            "count: {count} sur au moins {total} (temps écoulé : la recherche s’est arrêtée avant la fin, `total` est un minimum)\n\n",
            "count: {count} of at least {total} (timed out; the scan stopped early, so `total` is a lower bound)\n\n",
            count = count,
            total = total
        )
    } else if !l.total_exact {
        tr_in!(
            lang,
            "count: {count} sur au moins {total} (tronqué : la recherche s’est arrêtée à la limite, `total` est un minimum)\n\n",
            "count: {count} of at least {total} (truncated; the scan stopped at the limit, so `total` is a lower bound)\n\n",
            count = count,
            total = total
        )
    } else if l.truncated {
        tr_in!(
            lang,
            "count: {count} sur {total} (tronqué ; monte `limit` pour en voir plus)\n\n",
            "count: {count} of {total} (truncated; raise `limit` to see more)\n\n",
            count = count,
            total = total
        )
    } else {
        format!("count: {count}\n\n")
    };
    for m in &l.matches {
        match (ids, m.enclosing_id) {
            (true, Some(id)) => {
                let _ = writeln!(
                    text,
                    "{}:{}: {} [in {}, id: {id}]",
                    m.path,
                    m.line,
                    m.text,
                    m.enclosing.as_deref().unwrap_or_default()
                );
            }
            _ => {
                let _ = writeln!(text, "{}:{}: {}", m.path, m.line, m.text);
            }
        }
    }
    if l.timed_out {
        text.push_str(&tr_in!(
            lang,
            "note : la recherche a atteint son temps et s’est arrêtée avant la fin. Les résultats sont ceux trouvés jusque-là ; `total` est un minimum. Resserre-la avec `path_include`, ou monte `timeout_ms`.\n",
            "note: The scan reached its time budget and stopped early. The matches shown are the ones found so far; `total` is a lower bound, not the number of matches in the project. Narrow the search with `path_include`, or raise `timeout_ms`.\n"
        ));
    }
    truncate_response(&text)
}

/// The text format of `index_body` (tokensave `body_result`, `candidates_result`).
pub fn body_text(lang: Lang, b: &Body) -> String {
    let text = match b {
        Body::None { label } => tr_in!(
            lang,
            "Aucun symbole {label}.",
            "No symbol {label} found.",
            label = label
        ),
        Body::One { hit, body } => format!(
            "match_count: 1\n\nfile: {}:{}-{}: {} ({})\n{body}\n\n",
            hit.path, hit.line, hit.end_line, hit.name, hit.kind
        ),
        Body::Many {
            symbol,
            total,
            candidates,
        } => {
            let example = candidates
                .first()
                .map(|(h, _)| member_example(h))
                .unwrap_or_default();
            let hint = tr_in!(
                lang,
                "« {symbol} » est ambigu : {total} symboles lui répondent, donc aucun corps n’est rendu. Rappelle index_body avec un nom qualifié (par exemple symbol: \"{example}\" ; `Type.Member` marche aussi) ou avec node_id, l’id du candidat.",
                "'{symbol}' is ambiguous: {total} symbols match, so no body was returned. Re-call index_body with a qualified name (e.g. symbol: \"{example}\"; `Type.Member` also works) or with node_id set to the candidate's id.",
                symbol = symbol,
                total = total,
                example = example
            );
            let mut text = format!("match_count: 0\ncandidate_count: {total}\n{hint}\n\n");
            for (h, q) in candidates {
                let shown_q = q
                    .strip_prefix(&format!("{}::", h.path))
                    .unwrap_or(q.as_str());
                let _ = writeln!(
                    text,
                    "{} ({}) {}:{}-{} id={}",
                    shown_q, h.kind, h.path, h.line, h.end_line, h.id
                );
            }
            if *total > candidates.len() {
                let _ = writeln!(
                    text,
                    "... {} more (raise `limit`)",
                    total - candidates.len()
                );
            }
            text
        }
    };
    truncate_response(&text)
}

/// tokensave `truncate_response` for a text: cut at `MAX_RESPONSE_CHARS`, on a character.
pub fn truncate_response(s: &str) -> String {
    if s.len() <= MAX_RESPONSE_CHARS {
        return s.to_string();
    }
    let mut end = MAX_RESPONSE_CHARS;
    while !s.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    format!("{}\n\n[... truncated at {} chars]", &s[..end], end)
}

/// tokensave `truncate_response_keep_tail`: keeps everything from the last `marker` down when it
/// is short (2 000 characters at most).
pub fn truncate_response_keep_tail(s: &str, marker: &str) -> String {
    const MAX_TAIL_CHARS: usize = 2_000;
    if s.len() <= MAX_RESPONSE_CHARS {
        return s.to_string();
    }
    let Some(idx) = s.rfind(marker) else {
        return truncate_response(s);
    };
    let tail = &s[idx..];
    if tail.len() > MAX_TAIL_CHARS {
        return truncate_response(s);
    }
    let budget = MAX_RESPONSE_CHARS - tail.len();
    let mut end = budget.min(idx);
    while !s.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    format!(
        "{}\n\n[... truncated at {} chars]\n\n{}",
        &s[..end],
        end,
        tail
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codeindex::api::{Retrieval, Snippet, TermHits};

    // Ported from tokensave `src/context/formatter.rs`'s tests.

    fn make_test_context() -> Context {
        Context {
            entry_points: vec![],
            related: vec![],
            snippets: vec![],
            retrieval: Retrieval::default(),
        }
    }

    fn make_node(name: &str, path: &str) -> Hit {
        Hit {
            id: 7,
            name: name.into(),
            kind: "function".into(),
            parent: String::new(),
            path: path.into(),
            line: 2,
            end_line: 6,
            sig: String::new(),
            doc: String::new(),
        }
    }

    fn md(ctx: &Context) -> String {
        context_markdown(Lang::En, "test query", ctx)
    }

    #[test]
    fn test_markdown_contains_header() {
        let out = md(&make_test_context());
        assert!(out.contains("## Code Context"));
        assert!(out.contains("test query"));
    }

    #[test]
    fn test_markdown_with_entry_points() {
        let mut ctx = make_test_context();
        let mut n = make_node("process_data", "src/lib.rs");
        n.line = 11;
        n.sig = "pub fn process_data(input: &str) -> Result<()>".into();
        ctx.entry_points = vec![n];
        let out = md(&ctx);
        assert!(out.contains("**process_data**"));
        assert!(out.contains("(function)"));
        assert!(out.contains("src/lib.rs:11"));
        assert!(out.contains("`pub fn process_data(input: &str) -> Result<()>`"));
        assert!(out.contains("seen_node_ids: [7]"), "{out}");
    }

    #[test]
    fn test_markdown_with_code_blocks() {
        let mut ctx = make_test_context();
        ctx.entry_points = vec![make_node("my_fn", "src/main.rs")];
        ctx.snippets = vec![Snippet {
            id: 7,
            path: "src/main.rs".into(),
            line: 2,
            end_line: 4,
            text: "fn my_fn() {\n    println!(\"hello\");\n}".into(),
        }];
        let out = md(&ctx);
        assert!(out.contains("#### my_fn (src/main.rs:2)"));
        assert!(out.contains("```rust"));
        assert!(out.contains("fn my_fn()"));
    }

    #[test]
    fn test_retrieval_footer_reports_term_hits_and_zero_hit_warning() {
        let mut ctx = make_test_context();
        ctx.retrieval = Retrieval {
            terms: vec![
                TermHits {
                    term: "context".into(),
                    hits: 3,
                },
                TermHits {
                    term: "ranking".into(),
                    hits: 0,
                },
                TermHits {
                    term: "budget".into(),
                    hits: 0,
                },
            ],
            best_score: Some(21.5),
            quality: Some("exact".into()),
        };
        let out = md(&ctx);
        assert!(out.contains("### Retrieval"), "{out}");
        assert!(out.contains("- match: exact (best score 21.50)"), "{out}");
        assert!(out.contains("- terms: context(3)"), "{out}");
        assert!(out.contains("no hits (2): ranking, budget"), "{out}");
    }

    #[test]
    fn test_retrieval_footer_caps_zero_hit_term_list() {
        let mut ctx = make_test_context();
        ctx.retrieval.terms = (0..10)
            .map(|i| TermHits {
                term: format!("miss{i}"),
                hits: 0,
            })
            .collect();
        let out = md(&ctx);
        assert!(out.contains("no hits (10):"), "{out}");
        assert!(out.contains("+4 more"), "{out}");
        assert!(!out.contains("miss7"), "{out}");
    }

    #[test]
    fn test_retrieval_footer_tiny_score_shown_as_below_threshold() {
        let mut ctx = make_test_context();
        ctx.retrieval = Retrieval {
            terms: vec![TermHits {
                term: "foo".into(),
                hits: 1,
            }],
            best_score: Some(1e-6),
            quality: Some("fts-only".into()),
        };
        let out = md(&ctx);
        assert!(
            out.contains("- match: fts-only (best score <0.01)"),
            "{out}"
        );
        assert!(!out.contains("no hits"), "{out}");
    }

    #[test]
    fn test_retrieval_footer_absent_without_diagnostics() {
        let mut ctx = make_test_context();
        ctx.entry_points = vec![make_node("found_fn", "src/lib.rs")];
        assert!(!md(&ctx).contains("### Retrieval"));
    }

    #[test]
    fn test_retrieval_footer_present_when_no_entry_points_and_no_terms() {
        let out = md(&make_test_context());
        assert!(out.contains("### Retrieval"), "{out}");
        assert!(out.contains("no searchable terms extracted"), "{out}");
    }

    #[test]
    fn test_related_symbols_collapse_test_files() {
        let mut ctx = make_test_context();
        ctx.related = vec![
            make_node("real_fn", "src/lib.rs"),
            make_node("test_one", "tests/a_test.rs"),
            make_node("test_two", "tests/b_test.rs"),
        ];
        let out = md(&ctx);
        assert!(out.contains("- src/lib.rs: real_fn:2"), "{out}");
        assert!(!out.contains("test_one"), "{out}");
        assert!(
            out.contains("- test/fixture files: 2 symbols across 2 files"),
            "{out}"
        );
    }

    #[test]
    fn test_entry_point_docstring_first_line_shown() {
        let mut ctx = make_test_context();
        let mut node = make_node("documented", "src/lib.rs");
        node.doc = "Parses the config file.\n\nLong detail here.".into();
        ctx.entry_points = vec![node];
        let out = md(&ctx);
        assert!(out.contains("  Parses the config file."), "{out}");
        assert!(!out.contains("Long detail here"), "{out}");
    }

    #[test]
    fn test_fence_language_from_extension() {
        assert_eq!(fence_language("a/b.tsx"), "tsx");
        assert_eq!(fence_language("a/b.ts"), "typescript");
        assert_eq!(fence_language("a/b.rs"), "rust");
        assert_eq!(fence_language("lib/tasks/sample.rake"), "ruby");
        assert_eq!(fence_language("a/b.unknownext"), "");
    }

    #[test]
    fn compact_signature_leaves_a_normal_signature_alone() {
        let sig = "pub fn process_data(input: &str) -> Result<()>";
        assert_eq!(compact_signature(sig), sig);
    }

    #[test]
    fn compact_signature_collapses_a_multiline_initializer() {
        let sig = "MESSAGES = {\n  \"a\": \"one\",\n  \"b\": \"two\",\n}";
        assert_eq!(compact_signature(sig), "MESSAGES = { …");
    }

    #[test]
    fn compact_signature_bounds_a_long_single_line() {
        let sig = format!("const TABLE = [{}]", "0, ".repeat(500));
        let out = compact_signature(&sig);
        assert!(
            out.len() <= MAX_SIGNATURE_LEN + 8,
            "got {} bytes",
            out.len()
        );
        assert!(out.ends_with('…'));
    }

    #[test]
    fn compact_signature_truncates_on_a_char_boundary() {
        let sig = format!("const EMOJI = \"{}\"", "🚀".repeat(400));
        let out = compact_signature(&sig);
        assert!(out.ends_with('…'));
        assert!(out.chars().count() <= MAX_SIGNATURE_LEN + 2);
    }

    // Ours: the other formats.

    #[test]
    fn the_search_lists_one_line_per_symbol_and_says_when_it_was_cut() {
        let mut h = make_node("run", "src/a.rs");
        h.sig = "fn run()".into();
        let found = Found {
            hits: vec![h],
            truncated: true,
        };
        let out = search_text(Lang::En, &found, false);
        assert!(out.starts_with("count: 1 (truncated"), "{out}");
        assert!(
            out.contains("src/a.rs:2-6: run (function) — fn run()"),
            "{out}"
        );
        assert!(search_text(Lang::En, &found, true).contains("run (function) [id: 7] —"));
    }

    #[test]
    fn a_long_context_keeps_its_retrieval_tail() {
        let long = format!("{}### Retrieval\n- match: exact\n", "x".repeat(20_000));
        let out = truncate_response_keep_tail(&long, "### Retrieval");
        assert!(out.len() < 16_000);
        assert!(out.ends_with("### Retrieval\n- match: exact\n"), "{out}");
    }
}
