//! Searching the files of an editor source with `git grep`: what the editor's search shows, and
//! where Ctrl+click looks for a definition when no link leads anywhere.

use crate::git;
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

/// Never more matches than that, whatever is asked.
pub const MAX_RESULTS: usize = 5_000;
/// Past that, git is stopped and what it found is what there is.
const TIME_LIMIT: Duration = Duration::from_secs(10);
/// A matching line is cut there: a minified file's single line is not sent whole.
const MAX_TEXT_CHARS: usize = 300;
/// Where the agents' worktrees are, in the project's folder (or a folder of it).
const WORKTREES: &str = ".claude/worktrees";

/// What to look for: `pattern` as written, or an extended regular expression with `regex`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    pub pattern: String,
    pub regex: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub max_results: usize,
}

/// A line that matches: its file from the source's root (with `/`), its line and the column of
/// its first match (1-based, the column in characters), and its text, cut.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatch {
    pub path: String,
    pub line: u32,
    pub col: u32,
    pub text: String,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub matches: Vec<SearchMatch>,
    /// More lines matched than were asked for, or git was stopped at the time limit.
    pub truncated: bool,
}

/// The lines of the files of `root` matching `query`: those git knows and those it does not, the
/// ignored ones and the agents' worktrees left out.
pub async fn search(root: &str, query: &SearchQuery) -> Result<SearchResult> {
    search_within(root, query, TIME_LIMIT).await
}

/// `search`, git stopped past `limit` (a shorter one lets the stop be tested).
async fn search_within(root: &str, query: &SearchQuery, limit: Duration) -> Result<SearchResult> {
    if query.pattern.is_empty() {
        bail!("rien à chercher : le motif est vide");
    }
    let max = query.max_results.clamp(1, MAX_RESULTS);
    let args = grep_args(query, git::toplevel(root).await.is_some());
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let mut cmd = git::command(root, &args);
    cmd.kill_on_drop(true);
    crate::job::isolate(&mut cmd);
    let mut child = cmd.spawn()?;
    // Git for Windows' git.exe starts the real one: stopping it must stop both.
    let job = crate::job::Job::for_child(&child);
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("git grep : pas de sortie"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("git grep : pas de sortie d'erreur"))?;
    // Read meanwhile: a full pipe would block git before its end.
    let errors = tokio::spawn(async move {
        let mut b = Vec::new();
        let _ = stderr.read_to_end(&mut b).await;
        b
    });
    let mut out = SearchResult::default();
    let mut lines = BufReader::new(stdout);
    // True once one more line matched than are wanted.
    let read = async {
        let mut raw = Vec::new();
        loop {
            raw.clear();
            if lines.read_until(b'\n', &mut raw).await? == 0 {
                return Ok::<bool, std::io::Error>(false);
            }
            if let Some(m) = parse_line(&raw) {
                if out.matches.len() == max {
                    return Ok(true);
                }
                out.matches.push(m);
            }
        }
    };
    let more = tokio::time::timeout(limit, read)
        .await
        .unwrap_or(Ok(true))?;
    if more {
        if let Some(j) = &job {
            j.terminate();
        }
        let _ = child.kill().await;
        out.truncated = true;
        return Ok(out);
    }
    let status = child.wait().await?;
    // 1: nothing matched.
    if matches!(status.code(), Some(0 | 1)) {
        return Ok(out);
    }
    let errors = errors.await.unwrap_or_default();
    let msg = String::from_utf8_lossy(&errors).trim().to_string();
    bail!(if msg.is_empty() {
        "git grep a échoué".to_string()
    } else {
        msg
    })
}

/// The arguments of `git grep` for `query`, in a repository (`repo`) or a plain folder.
fn grep_args(query: &SearchQuery, repo: bool) -> Vec<String> {
    // -z: the fields of a line end with NUL, a path with `:` in it is read whole.
    let mut a: Vec<String> = ["grep", "-z", "-n", "--column", "-I", "--no-color"]
        .map(String::from)
        .into();
    a.push(if query.regex { "-E" } else { "-F" }.into());
    if !query.case_sensitive {
        a.push("-i".into());
    }
    if query.whole_word {
        a.push("-w".into());
    }
    if repo {
        // The files git knows and the new ones; the ignored ones stay out.
        a.push("--untracked".into());
    } else {
        a.extend(["--no-index".into(), "--exclude-standard".into()]);
    }
    // After -e, a pattern is never taken for an option (`--help`).
    a.extend(["-e".into(), query.pattern.clone(), "--".into()]);
    let out_of = |dir: &str| format!(":(exclude,glob)**/{dir}/**");
    a.push(out_of(WORKTREES));
    if !repo {
        // As the editor's tree walks such a folder.
        a.extend(git::WALK_SKIPPED.iter().map(|d| out_of(d)));
    }
    a
}

/// A line of `git grep -z -n --column`: `path NUL line NUL column NUL text`, the column counted
/// in bytes. None for what is not one.
fn parse_line(raw: &[u8]) -> Option<SearchMatch> {
    let raw = raw.strip_suffix(b"\n").unwrap_or(raw);
    let raw = raw.strip_suffix(b"\r").unwrap_or(raw);
    let mut fields = raw.splitn(4, |b| *b == 0);
    let path = String::from_utf8_lossy(fields.next()?).into_owned();
    let number = |f: &[u8]| std::str::from_utf8(f).ok()?.parse::<usize>().ok();
    let line = number(fields.next()?)?;
    let byte_col = number(fields.next()?)?;
    let text = fields.next()?;
    let before = &text[..byte_col.saturating_sub(1).min(text.len())];
    let col = String::from_utf8_lossy(before).chars().count() + 1;
    Some(SearchMatch {
        path,
        line: u32::try_from(line).ok()?,
        col: u32::try_from(col).ok()?,
        text: String::from_utf8_lossy(text)
            .chars()
            .take(MAX_TEXT_CHARS)
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::test_dir;
    use std::path::{Path, PathBuf};

    fn query(pattern: &str) -> SearchQuery {
        SearchQuery {
            pattern: pattern.into(),
            regex: false,
            case_sensitive: true,
            whole_word: false,
            max_results: 100,
        }
    }

    fn git(dir: &Path, args: &[&str]) {
        let ok = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?}");
    }

    /// A repository whose `src/app.ts` is committed, with `dist/` ignored.
    fn repo(name: &str) -> PathBuf {
        let r = test_dir(name);
        git(&r, &["init", "-q", "-b", "main"]);
        git(&r, &["config", "user.email", "t@t"]);
        git(&r, &["config", "user.name", "t"]);
        git(&r, &["config", "core.autocrlf", "false"]);
        std::fs::create_dir_all(r.join("src")).unwrap();
        std::fs::write(r.join("src/app.ts"), "const needle = 1;\n").unwrap();
        std::fs::write(r.join(".gitignore"), "dist/\n").unwrap();
        git(&r, &["add", "-A"]);
        git(&r, &["commit", "-qm", "init"]);
        r
    }

    fn write(root: &Path, rel: &str, text: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    async fn found(root: &Path, q: &SearchQuery) -> Vec<(String, u32)> {
        let r = search(&root.to_string_lossy(), q).await.unwrap();
        let mut out: Vec<_> = r.matches.into_iter().map(|m| (m.path, m.line)).collect();
        out.sort();
        out
    }

    fn at(path: &str, line: u32) -> (String, u32) {
        (path.to_string(), line)
    }

    #[tokio::test]
    async fn searches_the_files_git_knows_and_the_new_ones_but_not_the_ignored_ones() {
        let r = repo("search-tracked");
        write(&r, "notes.md", "a needle\n");
        write(&r, "dist/out.js", "needle\n");
        assert_eq!(
            found(&r, &query("needle")).await,
            vec![at("notes.md", 1), at("src/app.ts", 1)]
        );
    }

    #[tokio::test]
    async fn matches_whole_words_any_case_or_a_regular_expression_when_asked() {
        let r = repo("search-flags");
        write(&r, "f.txt", "foo\nfoobar\nFOO\nfo-o\n");
        let q = |pattern: &str, regex: bool, case_sensitive: bool, whole_word: bool| SearchQuery {
            pattern: pattern.into(),
            regex,
            case_sensitive,
            whole_word,
            max_results: 100,
        };
        assert_eq!(
            found(&r, &q("foo", false, true, false)).await,
            vec![at("f.txt", 1), at("f.txt", 2)]
        );
        assert_eq!(
            found(&r, &q("foo", false, true, true)).await,
            vec![at("f.txt", 1)]
        );
        assert_eq!(
            found(&r, &q("foo", false, false, true)).await,
            vec![at("f.txt", 1), at("f.txt", 3)]
        );
        // As written unless asked: `.` and `+` are no operators.
        assert_eq!(found(&r, &q("fo+b", false, true, false)).await, vec![]);
        assert_eq!(
            found(&r, &q("fo+b", true, true, false)).await,
            vec![at("f.txt", 2)]
        );
        assert_eq!(
            found(&r, &q("^f.-o$|^FO+$", true, true, false)).await,
            vec![at("f.txt", 3), at("f.txt", 4)]
        );
    }

    #[tokio::test]
    async fn stops_at_the_number_of_matches_asked_and_says_there_are_more() {
        let r = repo("search-cap");
        write(&r, "many.txt", &"hit\n".repeat(5));
        let mut q = query("hit");
        q.max_results = 3;
        let res = search(&r.to_string_lossy(), &q).await.unwrap();
        assert_eq!((res.matches.len(), res.truncated), (3, true));
        q.max_results = 5;
        let res = search(&r.to_string_lossy(), &q).await.unwrap();
        assert_eq!((res.matches.len(), res.truncated), (5, false));
    }

    #[tokio::test]
    async fn never_gives_more_than_the_hard_bound_whatever_is_asked() {
        let r = repo("search-bound");
        write(&r, "many.txt", &"hit\n".repeat(MAX_RESULTS + 1));
        let mut q = query("hit");
        q.max_results = usize::MAX;
        let res = search(&r.to_string_lossy(), &q).await.unwrap();
        assert_eq!((res.matches.len(), res.truncated), (MAX_RESULTS, true));
    }

    #[tokio::test]
    async fn stops_git_at_the_time_limit_and_says_there_may_be_more() {
        let r = repo("search-time");
        let res = search_within(&r.to_string_lossy(), &query("needle"), Duration::ZERO)
            .await
            .unwrap();
        assert!(res.truncated);
        // In time, the same search finds it all.
        let res = search(&r.to_string_lossy(), &query("needle"))
            .await
            .unwrap();
        assert_eq!((res.matches.len(), res.truncated), (1, false));
    }

    #[tokio::test]
    async fn finding_nothing_is_an_empty_list_not_an_error() {
        let r = repo("search-none");
        let res = search(&r.to_string_lossy(), &query("absent-everywhere"))
            .await
            .unwrap();
        assert_eq!(res, SearchResult::default());
    }

    #[tokio::test]
    async fn refuses_an_empty_pattern_and_tells_a_wrong_expression() {
        let r = repo("search-refused");
        assert!(search(&r.to_string_lossy(), &query("")).await.is_err());
        let mut q = query("a(");
        q.regex = true;
        assert!(search(&r.to_string_lossy(), &q).await.is_err());
    }

    #[tokio::test]
    async fn looks_for_a_pattern_that_reads_like_an_option_as_written() {
        let r = repo("search-option");
        write(&r, "cli.md", "run it with --help\n-e alone\n");
        assert_eq!(found(&r, &query("--help")).await, vec![at("cli.md", 1)]);
        assert_eq!(found(&r, &query("-e")).await, vec![at("cli.md", 2)]);
    }

    #[tokio::test]
    async fn searches_a_folder_that_is_not_a_repository() {
        let d = test_dir("search-plain");
        write(&d, "a.ts", "needle\n");
        write(&d, "sub/b.ts", "needle\n");
        // What the editor's tree of such a folder leaves out: ignored files, installs, builds.
        write(&d, ".gitignore", "secret.txt\n");
        write(&d, "secret.txt", "needle\n");
        write(&d, "node_modules/x/i.js", "needle\n");
        write(&d, "target/debug/out.txt", "needle\n");
        assert_eq!(
            found(&d, &query("needle")).await,
            vec![at("a.ts", 1), at("sub/b.ts", 1)]
        );
    }

    #[tokio::test]
    async fn leaves_the_agents_worktrees_out_of_the_projects_search() {
        let r = repo("search-worktrees");
        // Neither excluded nor a checkout of its own here: only the search leaves it out.
        write(
            &r,
            ".claude/worktrees/refacto/src/app.ts",
            "const needle = 2;\n",
        );
        write(&r, "sub/.claude/worktrees/other/x.ts", "needle\n");
        write(&r, ".claude/settings.json", "needle\n");
        assert_eq!(
            found(&r, &query("needle")).await,
            vec![at(".claude/settings.json", 1), at("src/app.ts", 1)]
        );
        // The worktree's own search finds its files.
        let wt = r.join(".claude/worktrees/refacto");
        git(&wt, &["init", "-q", "-b", "main"]);
        assert_eq!(
            found(&wt, &query("needle")).await,
            vec![at("src/app.ts", 1)]
        );
    }

    #[tokio::test]
    async fn counts_the_column_in_characters_and_cuts_a_long_line() {
        let r = repo("search-column");
        write(
            &r,
            "fr.md",
            &format!("é à needle\r\n{}needle\n", "ü".repeat(400)),
        );
        let res = search(&r.to_string_lossy(), &query("needle"))
            .await
            .unwrap();
        let m = &res.matches[0];
        assert_eq!((m.line, m.col, m.text.as_str()), (1, 5, "é à needle"));
        let m = &res.matches[1];
        assert_eq!((m.line, m.col), (2, 401));
        assert_eq!(m.text, "ü".repeat(300));
    }
}
