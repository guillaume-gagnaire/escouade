//! Ported from tokensave `src/config.rs` (`QueryIgnore`, `load_query_ignore`) and the path filters
//! of `src/context/builder.rs` (`path_lists_keep`, the `path_prefix` filter) and
//! `src/mcp/tools/handlers/mod.rs` (`filter_by_path_lists`) (github.com/aovestdipaperino/tokensave,
//! commit 580936b, MIT, © 2025 Enzo Lombardi; see THIRD-PARTY-NOTICES.md): which results a query
//! keeps, by their path.
//!
//! What differs, for our architecture: the file of the paths ignored by the queries is
//! `.escouade/queryignore` (tokensave: `.tokensave/queryignore`), read in the folder asked (the
//! agent's worktree, as committed there), and never through a link that leads out of that folder,
//! nor when the project's fence refuses it.

use glob::Pattern;
use std::path::Path;

/// The file of the paths ignored by the queries, under `.escouade/`.
pub const QUERYIGNORE_FILENAME: &str = "queryignore";

#[derive(Debug, Clone)]
enum IgnoreRule {
    Glob(Pattern),
    Substring(String),
}

impl IgnoreRule {
    fn matches(&self, path: &str) -> bool {
        match self {
            IgnoreRule::Glob(pattern) => pattern.matches_with(
                path,
                glob::MatchOptions {
                    case_sensitive: true,
                    require_literal_separator: false,
                    require_literal_leading_dot: false,
                },
            ),
            IgnoreRule::Substring(needle) => path.contains(needle),
        }
    }
}

/// Project-level set of query-time ignore patterns: once configured in `.escouade/queryignore`,
/// matching results are dropped from `index_search` and `index_context` without the caller
/// having to pass a filter on every request. They do not affect indexing.
#[derive(Debug, Clone, Default)]
pub struct QueryIgnore {
    rules: Vec<IgnoreRule>,
}

impl QueryIgnore {
    /// One pattern per line. Blank lines and lines whose first non-whitespace character is `#`
    /// are ignored. A pattern containing `*` is a glob; everything else is a substring match.
    /// Invalid globs are silently skipped.
    pub fn parse(contents: &str) -> Self {
        let mut rules = Vec::new();
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let normalized = trimmed.replace('\\', "/");
            if normalized.contains('*') {
                if let Ok(pattern) = Pattern::new(&normalized) {
                    rules.push(IgnoreRule::Glob(pattern));
                }
            } else {
                rules.push(IgnoreRule::Substring(normalized));
            }
        }
        QueryIgnore { rules }
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Whether `file_path` (normalized to `/`) matches any pattern.
    pub fn is_ignored(&self, file_path: &str) -> bool {
        if self.rules.is_empty() {
            return false;
        }
        let normalized = file_path.replace('\\', "/");
        self.rules.iter().any(|rule| rule.matches(&normalized))
    }
}

/// The patterns of `<dir>/.escouade/queryignore`: none when the file is absent, unreadable, or
/// reached through a link that leads out of `dir`.
pub fn load_query_ignore(dir: &Path) -> QueryIgnore {
    let path = dir.join(crate::docs::ROOT).join(QUERYIGNORE_FILENAME);
    let (Ok(real), Ok(root)) = (std::fs::canonicalize(&path), std::fs::canonicalize(dir)) else {
        return QueryIgnore::default();
    };
    if !real.starts_with(&root) {
        return QueryIgnore::default();
    }
    match std::fs::read_to_string(&real) {
        Ok(contents) => QueryIgnore::parse(&contents),
        Err(_) => QueryIgnore::default(),
    }
}

/// What a query keeps by path: `path_include`, `path_exclude` (substrings, the exclusion first),
/// `path` (a folder or a file), and the patterns of `.escouade/queryignore`.
#[derive(Debug, Clone, Default)]
pub struct PathFilter {
    pub prefix: Option<String>,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub ignore: QueryIgnore,
}

impl PathFilter {
    pub fn new(
        prefix: Option<&str>,
        include: Vec<String>,
        exclude: Vec<String>,
        ignore: QueryIgnore,
    ) -> Self {
        let clean = |v: Vec<String>| -> Vec<String> {
            v.into_iter()
                .map(|s| s.trim().replace('\\', "/"))
                .filter(|s| !s.is_empty())
                .collect()
        };
        PathFilter {
            prefix: prefix
                .map(|p| p.trim().replace('\\', "/"))
                .filter(|p| !p.is_empty()),
            include: clean(include),
            exclude: clean(exclude),
            ignore,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.prefix.is_none()
            && self.include.is_empty()
            && self.exclude.is_empty()
            && self.ignore.is_empty()
    }

    /// Whether a result of the file `path` is kept.
    pub fn keeps(&self, path: &str) -> bool {
        path_lists_keep(path, &self.include, &self.exclude)
            && !self.ignore.is_ignored(path)
            && self
                .prefix
                .as_deref()
                .is_none_or(|prefix| prefix_keeps(path, prefix))
    }
}

/// tokensave `path_lists_keep`: `exclude` takes precedence over `include`.
pub fn path_lists_keep(path: &str, include: &[String], exclude: &[String]) -> bool {
    let normalized = path.replace('\\', "/");
    if exclude.iter().any(|sub| normalized.contains(sub.as_str())) {
        return false;
    }
    if !include.is_empty() {
        return include.iter().any(|sub| normalized.contains(sub.as_str()));
    }
    true
}

/// The `path_prefix` filter: the file itself, or a file under the folder.
pub fn prefix_keeps(path: &str, prefix: &str) -> bool {
    let with_slash = format!("{}/", prefix.trim_end_matches('/'));
    path == prefix || path.starts_with(&with_slash)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ported from tokensave `src/config.rs`'s tests.

    #[test]
    fn test_query_ignore_substring_match() {
        let qi = QueryIgnore::parse("generated\n");
        assert!(qi.is_ignored("src/generated/api.rs"));
        assert!(qi.is_ignored("generated.rs"));
        assert!(!qi.is_ignored("src/main.rs"));
    }

    #[test]
    fn test_query_ignore_glob_match() {
        let qi = QueryIgnore::parse("tests/*\n**/proto/**\n");
        assert!(qi.is_ignored("tests/foo.rs"));
        assert!(qi.is_ignored("tests/sub/bar.rs"));
        assert!(qi.is_ignored("crate/proto/messages.rs"));
        assert!(!qi.is_ignored("src/lib.rs"));
    }

    #[test]
    fn test_query_ignore_skips_comments_and_blanks() {
        let qi = QueryIgnore::parse("# a comment\n\n   \n  vendor  \n");
        assert!(qi.is_ignored("third_party/vendor/lib.rs"));
        assert!(!qi.is_ignored("src/main.rs"));
    }

    #[test]
    fn test_query_ignore_empty_matches_nothing() {
        let qi = QueryIgnore::default();
        assert!(qi.is_empty());
        assert!(!qi.is_ignored("anything/at/all.rs"));
        assert!(QueryIgnore::parse("# only comments\n\n").is_empty());
    }

    #[test]
    fn test_query_ignore_normalizes_separators() {
        let qi = QueryIgnore::parse("src/gen\n");
        assert!(qi.is_ignored("src\\gen\\out.rs"));
    }

    #[test]
    fn test_load_query_ignore_absent_is_empty() {
        let dir = crate::paths::test_dir("queryignore-absent");
        assert!(load_query_ignore(&dir).is_empty());
    }

    #[test]
    fn test_load_query_ignore_reads_file() {
        let dir = crate::paths::test_dir("queryignore-reads");
        let at = dir.join(".escouade");
        std::fs::create_dir_all(&at).unwrap();
        std::fs::write(at.join("queryignore"), "generated\ntests/*\n").unwrap();
        let qi = load_query_ignore(&dir);
        assert!(!qi.is_empty());
        assert!(qi.is_ignored("src/generated/x.rs"));
        assert!(qi.is_ignored("tests/foo.rs"));
        assert!(!qi.is_ignored("src/main.rs"));
    }

    // Ours.

    #[cfg(unix)]
    #[test]
    fn a_queryignore_linked_out_of_the_folder_is_not_read() {
        let out = crate::paths::test_dir("queryignore-out");
        std::fs::write(out.join("queryignore"), "src\n").unwrap();
        let dir = crate::paths::test_dir("queryignore-linked");
        std::fs::create_dir_all(dir.join(".escouade")).unwrap();
        std::os::unix::fs::symlink(out.join("queryignore"), dir.join(".escouade/queryignore"))
            .unwrap();
        assert!(load_query_ignore(&dir).is_empty());
    }

    #[test]
    fn the_path_filter_keeps_by_prefix_lists_and_patterns() {
        let f = PathFilter::new(
            Some("src\\"),
            vec!["codeindex".into()],
            vec!["tests".into()],
            QueryIgnore::parse("*/probes/*\n"),
        );
        assert!(f.keeps("src/codeindex/api.rs"));
        assert!(!f.keeps("src/mcp/index.rs"), "not included");
        assert!(!f.keeps("src/codeindex/tests.rs"), "excluded");
        assert!(!f.keeps("src/codeindex/probes/mod.rs"), "ignored");
        assert!(!f.keeps("docs/codeindex.md"), "out of the prefix");
        assert!(PathFilter::default().is_empty());
        assert!(prefix_keeps("src/a.rs", "src/a.rs"));
        assert!(!prefix_keeps("srcx/a.rs", "src"));
    }
}
