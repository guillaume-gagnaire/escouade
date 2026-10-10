//! Searching the files of an editor source with `git grep`: what the editor's search shows, and
//! where Ctrl+click looks for a definition when no link leads anywhere.

use crate::git;
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::sync::OnceCell;

/// Never more matches than that, whatever is asked.
pub const MAX_RESULTS: usize = 5_000;
/// Past that, git is stopped and what it found is what there is.
const TIME_LIMIT: Duration = Duration::from_secs(10);
/// What is shown of a matching line: a minified file's single line is not sent whole.
const MAX_TEXT_CHARS: usize = 300;
/// How much of a longer line is shown before its first match.
const CONTEXT_CHARS: usize = 40;
/// Where the agents' worktrees are, in the project's folder (or a folder of it).
const WORKTREES: &str = ".claude/worktrees";
/// How git is told to mark each match of a line (`--color=always`, every other color off): an
/// SGR sequence no file is likely to hold, then the reset.
const MARK: &str = "reverse blink strike";
const MARK_ON: &[u8] = b"\x1b[5;7;9m";
const MARK_OFF: &[u8] = b"\x1b[m";

/// What to look for: `pattern` as written, or with `regex` an expression as JavaScript writes
/// them (read by git's Perl expressions when it has them, else translated for its extended ones).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    pub pattern: String,
    pub regex: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub max_results: usize,
}

/// A line that matches: its file from the source's root (with `/`), its line, the column of its
/// first match (1-based, in characters), and what to show of it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatch {
    pub path: String,
    pub line: u32,
    pub col: u32,
    /// At most 300 characters of the line: all of it, or a stretch starting 40 characters before
    /// its first match.
    pub text: String,
    /// Where `text` starts in the line, in characters (0 for the line's start).
    pub offset: u32,
    /// Each match in `text`, `[start, end)` in characters of `text`.
    pub ranges: Vec<[u32; 2]>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub matches: Vec<SearchMatch>,
    /// More lines matched than were asked for, or git was stopped at the time limit.
    pub truncated: bool,
    /// The time limit stopped git (`truncated` is set too).
    pub timed_out: bool,
}

/// The word boundaries of git's extended expressions, as its regex library has them.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
enum Ere {
    /// None.
    #[default]
    Posix,
    /// `\b` and `\B`: GNU's (Git for Windows, Linux).
    Gnu,
    /// `[[:<:]]` and `[[:>:]]`: BSD's (macOS).
    Bsd,
}

/// How this machine's git reads expressions: `-P` (Perl's, close to JavaScript's) when it has
/// them, else `-E` with the boundaries `ere`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Engine {
    perl: bool,
    ere: Ere,
}

static ENGINE: OnceCell<Engine> = OnceCell::const_new();

/// The engine of this machine's git, found out at the first search for an expression.
async fn engine() -> Engine {
    cached(&ENGINE, || async { probe_in(&std::env::temp_dir()).await }).await
}

/// The engine `cell` holds, else the one `probe` finds. A sure answer is kept; without one (git or
/// the temporary folder failing now), the default is used for this search and the next one tries
/// again.
async fn cached<F, Fut>(cell: &OnceCell<Engine>, probe: F) -> Engine
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Option<Engine>>,
{
    cell.get_or_try_init(|| async { probe().await.ok_or(()) })
        .await
        .copied()
        .unwrap_or_default()
}

/// A new folder of `parent`, made by this call under a name no one can guess: never one already
/// there, which another user of a shared /tmp may have made with links in it.
fn fresh_dir(parent: &Path) -> std::io::Result<PathBuf> {
    use std::hash::{BuildHasher, Hasher};
    for _ in 0..8 {
        // The keys of a RandomState come from the system's random source.
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u32(std::process::id());
        let dir = parent.join(format!("escouade-grep-probe-{:016x}", h.finish()));
        match private_dir(&dir) {
            Ok(()) => return Ok(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "pas de dossier libre pour essayer git grep",
    ))
}

/// Makes the folder `dir`, refused when something is there; on Unix, for its owner alone.
fn private_dir(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new().mode(0o700).create(dir)
    }
    #[cfg(not(unix))]
    std::fs::create_dir(dir)
}

/// What git's expressions find in a new folder of `parent` holding `a x`; None when that cannot
/// be told (no folder, no git).
async fn probe_in(parent: &Path) -> Option<Engine> {
    let dir = fresh_dir(parent).ok()?;
    let engine = probe_dir(&dir).await;
    let _ = std::fs::remove_dir_all(&dir);
    engine
}

async fn probe_dir(dir: &Path) -> Option<Engine> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join("p.txt"))
        .ok()?;
    file.write_all(b"a x\n").ok()?;
    drop(file);
    let root = dir.to_string_lossy();
    let ere = if finds(&root, "-E", r"\bx").await? {
        Ere::Gnu
    } else if finds(&root, "-E", "[[:<:]]x").await? {
        Ere::Bsd
    } else {
        Ere::Posix
    };
    Some(Engine {
        perl: finds(&root, "-P", r"\bx").await?,
        ere,
    })
}

/// Whether `pattern` (read as `flag` says) finds `x` in the folder `root`, by git's exit code alone
/// (its messages may be translated): 128 is a pattern it cannot read. None for another answer.
async fn finds(root: &str, flag: &str, pattern: &str) -> Option<bool> {
    let args = ["grep", "--no-index", "-q", flag, "-e", pattern];
    let out = git::command(root, &args).output().await.ok()?;
    match out.status.code() {
        Some(0) => Some(true),
        Some(1 | 128) => Some(false),
        _ => None,
    }
}

fn unsupported(what: &str) -> anyhow::Error {
    anyhow!("Cette expression n'est pas prise en charge par git sur ce poste : {what}")
}

/// What came before a `\b`, to tell a start of word from an end on BSD.
#[derive(Clone, Copy)]
enum Prev {
    /// Nothing, or the start of a group or of an alternative.
    Start,
    /// A character of a word.
    Word,
    Other,
}

/// `pattern`, as JavaScript writes expressions, for git's extended ones: `\d`, `\s`, `\w` and their
/// opposites as POSIX classes (in a set too), `\b` and `\B` as `ere` has them. What cannot be
/// said so is refused.
fn to_ere(pattern: &str, ere: Ere) -> Result<String> {
    let c: Vec<char> = pattern.chars().collect();
    let mut out = String::with_capacity(pattern.len());
    let mut prev = Prev::Start;
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch == '[' {
            let (set, next) = to_posix_set(&c, i)?;
            out.push_str(&set);
            prev = Prev::Other;
            i = next;
            continue;
        }
        if ch != '\\' || i + 1 == c.len() {
            out.push(ch);
            prev = if ch.is_alphanumeric() || ch == '_' {
                Prev::Word
            } else if matches!(ch, '(' | '|' | '^') {
                Prev::Start
            } else {
                Prev::Other
            };
            i += 1;
            continue;
        }
        let e = c[i + 1];
        i += 2;
        prev = match e {
            'd' | 'w' => {
                out.push_str(if e == 'd' { "[0-9]" } else { "[[:alnum:]_]" });
                Prev::Word
            }
            'D' => {
                out.push_str("[^0-9]");
                Prev::Other
            }
            's' => {
                out.push_str("[[:space:]]");
                Prev::Other
            }
            'S' => {
                out.push_str("[^[:space:]]");
                Prev::Other
            }
            'W' => {
                out.push_str("[^[:alnum:]_]");
                Prev::Other
            }
            't' => {
                out.push('\t');
                Prev::Other
            }
            'b' | 'B' => {
                let b = boundary(e, prev, &c[i..], ere)
                    .ok_or_else(|| unsupported(&format!("\\{e}")))?;
                out.push_str(b);
                // It takes no character: what comes next still follows `prev`.
                prev
            }
            other => {
                out.push('\\');
                out.push(other);
                Prev::Other
            }
        };
    }
    Ok(out)
}

/// A word boundary (`b`) or its opposite (`B`) in `ere`, after `prev` and before `rest`; None when
/// `ere` cannot say it there.
fn boundary(kind: char, prev: Prev, rest: &[char], ere: Ere) -> Option<&'static str> {
    match (ere, kind) {
        (Ere::Gnu, 'b') => Some(r"\b"),
        (Ere::Gnu, _) => Some(r"\B"),
        // BSD has a start and an end of word, not a boundary: which one follows from around it.
        (Ere::Bsd, 'b') => {
            let word_next = match rest {
                ['\\', 'w' | 'd', ..] => true,
                [c, ..] => c.is_alphanumeric() || *c == '_',
                [] => false,
            };
            match prev {
                Prev::Word => Some("[[:>:]]"),
                Prev::Start => Some("[[:<:]]"),
                Prev::Other if word_next => Some("[[:<:]]"),
                Prev::Other => None,
            }
        }
        _ => None,
    }
}

/// The set of characters at `c[at]` (`[…]`) as POSIX writes them (a backslash is no escape there:
/// `]` goes first, `^` and `-` last), and where it ends.
fn to_posix_set(c: &[char], at: usize) -> Result<(String, usize)> {
    let mut i = at + 1;
    let negated = c.get(i) == Some(&'^');
    if negated {
        i += 1;
    }
    let mut items = String::new();
    let (mut bracket, mut caret, mut dash) = (false, false, false);
    loop {
        let Some(&ch) = c.get(i) else {
            // Not closed: as written, for git to tell.
            return Ok((c[at..].iter().collect(), c.len()));
        };
        i += 1;
        match ch {
            ']' => break,
            // A class of POSIX's, as written for git (`[[:space:]]`): kept whole.
            '[' if c.get(i) == Some(&':') => {
                match (i + 1..c.len()).find(|&j| c[j] == ':' && c.get(j + 1) == Some(&']')) {
                    Some(j) => {
                        items.extend(c[i - 1..j + 2].iter());
                        i = j + 2;
                    }
                    None => items.push('['),
                }
            }
            '\\' if i < c.len() => {
                let e = c[i];
                i += 1;
                match e {
                    'd' => items.push_str("0-9"),
                    's' => items.push_str("[:space:]"),
                    'w' => items.push_str("[:alnum:]_"),
                    't' => items.push('\t'),
                    ']' => bracket = true,
                    '^' => caret = true,
                    '-' => dash = true,
                    'D' | 'S' | 'W' | 'b' | 'B' => return Err(unsupported(&format!("[\\{e}]"))),
                    other => items.push(other),
                }
            }
            other => items.push(other),
        }
    }
    let mut out = String::from(if negated { "[^" } else { "[" });
    if bracket {
        out.push(']');
    }
    out.push_str(&items);
    if caret {
        out.push('^');
    }
    if dash {
        out.push('-');
    }
    out.push(']');
    Ok((out, i))
}

/// The lines of the files of `root` matching `query`: those git knows and those it does not, the
/// ignored ones and the agents' worktrees left out.
pub async fn search(root: &str, query: &SearchQuery) -> Result<SearchResult> {
    search_within(root, query, TIME_LIMIT).await
}

/// `search`, git stopped past `limit` (a shorter one lets the stop be tested).
async fn search_within(root: &str, query: &SearchQuery, limit: Duration) -> Result<SearchResult> {
    let engine = if query.regex {
        engine().await
    } else {
        Engine::default()
    };
    run(root, query, limit, engine).await
}

/// `search` with the expressions of `engine` (another one than this git's lets each be tested).
async fn run(
    root: &str,
    query: &SearchQuery,
    limit: Duration,
    engine: Engine,
) -> Result<SearchResult> {
    if query.pattern.is_empty() {
        bail!("rien à chercher : le motif est vide");
    }
    // git would take each line for a pattern of its own, an empty one matching every line.
    if query.pattern.contains(['\n', '\r']) {
        bail!("un motif tient sur une seule ligne");
    }
    let pattern = if query.regex && !engine.perl {
        to_ere(&query.pattern, engine.ere)?
    } else {
        query.pattern.clone()
    };
    let max = query.max_results.clamp(1, MAX_RESULTS);
    let repo = git::toplevel(root).await.is_some();
    let args = grep_args(query, &pattern, engine.perl, repo);
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
    let read = tokio::time::timeout(limit, read).await;
    out.timed_out = read.is_err();
    if read.unwrap_or(Ok(true))? {
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

/// The arguments of `git grep` for `query`, its pattern being `pattern` (for `-P` when `perl`), in
/// a repository (`repo`) or a plain folder.
fn grep_args(query: &SearchQuery, pattern: &str, perl: bool, repo: bool) -> Vec<String> {
    let mut a = Vec::new();
    // Each match marked and nothing else colored, whatever the user's settings say.
    for slot in [
        "filename",
        "lineNumber",
        "column",
        "separator",
        "selected",
        "context",
        "function",
    ] {
        a.extend(["-c".to_string(), format!("color.grep.{slot}=")]);
    }
    a.extend(["-c".to_string(), format!("color.grep.match={MARK}")]);
    // --no-recurse-submodules: with `submodule.recurse` set, git refuses --untracked.
    // -z: the fields of a line end with NUL, a path with `:` in it is read whole.
    a.extend(
        [
            "grep",
            "--no-recurse-submodules",
            "--color=always",
            "-z",
            "-n",
            "--column",
            "-I",
        ]
        .map(String::from),
    );
    a.push(
        if !query.regex {
            "-F"
        } else if perl {
            "-P"
        } else {
            "-E"
        }
        .into(),
    );
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
    a.extend(["-e".into(), pattern.to_string(), "--".into()]);
    let out_of = |dir: &str| format!(":(exclude,glob)**/{dir}/**");
    a.push(out_of(WORKTREES));
    if !repo {
        // As the editor's tree walks such a folder.
        a.extend(git::WALK_SKIPPED.iter().map(|d| out_of(d)));
    }
    a
}

/// The text of a line git marked, where each match is in it (in bytes), and whether the marks can
/// be taken: one that does not pair up is the line's own (a log's colors), and then the spans are
/// not git's matches.
fn unmark(marked: &[u8]) -> (Vec<u8>, Vec<(usize, usize)>, bool) {
    let mut text = Vec::with_capacity(marked.len());
    let mut spans = Vec::new();
    let mut open: Option<usize> = None;
    let mut sound = true;
    let mut i = 0;
    while i < marked.len() {
        let rest = &marked[i..];
        let mark = [MARK_ON, MARK_OFF]
            .into_iter()
            .find(|m| rest.starts_with(m));
        match (mark, open) {
            (Some(MARK_ON), None) => open = Some(text.len()),
            (Some(MARK_OFF), Some(start)) => {
                spans.push((start, text.len()));
                open = None;
            }
            // Out of place: kept as text.
            (Some(m), _) => {
                sound = false;
                text.extend_from_slice(m);
            }
            (None, _) => {
                text.push(marked[i]);
                i += 1;
                continue;
            }
        }
        i += mark.map_or(0, <[u8]>::len);
    }
    (text, spans, sound && open.is_none())
}

/// `bytes` decoded (what is not UTF-8 replaced by U+FFFD), its length in characters, and the
/// character index of each byte offset of `at`: an offset inside a character is that character's.
fn decode(bytes: &[u8], at: &[usize]) -> (String, usize, Vec<usize>) {
    let mut order: Vec<usize> = (0..at.len()).collect();
    order.sort_by_key(|&k| at[k]);
    let mut order = order.into_iter().peekable();
    let mut index = vec![0; at.len()];
    let mut text = String::with_capacity(bytes.len());
    let (mut byte, mut chars) = (0, 0);
    // Each character with how many bytes it takes: an invalid run makes one.
    let pieces = bytes.utf8_chunks().flat_map(|chunk| {
        let bad = chunk.invalid().len();
        let bad = (bad > 0).then_some((char::REPLACEMENT_CHARACTER, bad));
        chunk.valid().chars().map(|c| (c, c.len_utf8())).chain(bad)
    });
    for (c, len) in pieces {
        while let Some(k) = order.next_if(|&k| at[k] < byte + len) {
            index[k] = chars;
        }
        text.push(c);
        byte += len;
        chars += 1;
    }
    // At the end, or past it.
    for k in order {
        index[k] = chars;
    }
    (text, chars, index)
}

/// A line of `git grep -z -n --column --color=always`: `path NUL line NUL column NUL text`, the
/// column counted in bytes, each match of the text marked. None for what is not one.
fn parse_line(raw: &[u8]) -> Option<SearchMatch> {
    let raw = raw.strip_suffix(b"\n").unwrap_or(raw);
    let mut fields = raw.splitn(4, |b| *b == 0);
    let path = String::from_utf8_lossy(fields.next()?).into_owned();
    let number = |f: &[u8]| std::str::from_utf8(f).ok()?.parse::<usize>().ok();
    let line = number(fields.next()?)?;
    let byte_col = number(fields.next()?)?;
    let (mut bytes, mut spans, sound) = unmark(fields.next()?);
    if bytes.last() == Some(&b'\r') {
        bytes.pop();
        let end = bytes.len();
        spans
            .iter_mut()
            .for_each(|s| *s = (s.0.min(end), s.1.min(end)));
    }
    // Decoded whole: a git comparing bytes may end or begin a match inside a character.
    let mut at: Vec<usize> = spans.iter().flat_map(|&(s, e)| [s, e]).collect();
    at.push(byte_col.saturating_sub(1).min(bytes.len()));
    let (text, n, index) = decode(&bytes, &at);
    let (&first, ends) = index.split_last()?;
    let spans: Vec<(usize, usize)> = ends.chunks(2).map(|c| (c[0], c[1])).collect();
    // The marks are taken when they agree with git's column; else the line is shown without them.
    let spans = if sound && spans.first().is_some_and(|s| s.0 == first) {
        spans
    } else {
        Vec::new()
    };
    // The whole line, or 300 characters of it from a little before its first match.
    let start = if n <= MAX_TEXT_CHARS {
        0
    } else {
        first.saturating_sub(CONTEXT_CHARS).min(n - MAX_TEXT_CHARS)
    };
    let end = (start + MAX_TEXT_CHARS).min(n);
    let c32 = |v: usize| u32::try_from(v).ok();
    let mut ranges = Vec::new();
    for &(s, e) in &spans {
        if e > s && s < end && e > start {
            ranges.push([c32(s.max(start) - start)?, c32(e.min(end) - start)?]);
        }
    }
    Some(SearchMatch {
        path,
        line: c32(line)?,
        col: c32(first + 1)?,
        text: text.chars().skip(start).take(end - start).collect(),
        offset: c32(start)?,
        ranges,
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
        assert!(res.truncated && res.timed_out);
        // In time, the same search finds it all.
        let res = search(&r.to_string_lossy(), &query("needle"))
            .await
            .unwrap();
        assert_eq!(
            (res.matches.len(), res.truncated, res.timed_out),
            (1, false, false)
        );
    }

    #[tokio::test]
    async fn a_search_cut_at_the_number_asked_did_not_run_out_of_time() {
        let r = repo("search-cap-time");
        write(&r, "many.txt", &"hit\n".repeat(5));
        let mut q = query("hit");
        q.max_results = 2;
        let res = search(&r.to_string_lossy(), &q).await.unwrap();
        assert_eq!((res.truncated, res.timed_out), (true, false));
    }

    #[tokio::test]
    async fn searches_a_repository_whose_git_goes_into_submodules() {
        let r = repo("search-submodules");
        // A common global setting, with which git refuses `--untracked`.
        git(&r, &["config", "submodule.recurse", "true"]);
        assert_eq!(found(&r, &query("needle")).await, vec![at("src/app.ts", 1)]);
    }

    #[tokio::test]
    async fn refuses_a_pattern_of_several_lines() {
        // git would take each line for a pattern, an empty one matching every line.
        let r = repo("search-lines");
        for p in ["needle\n", "x\ny", "needle\r"] {
            let e = search(&r.to_string_lossy(), &query(p)).await.unwrap_err();
            assert!(e.to_string().contains("ligne"), "{p:?}: {e}");
        }
    }

    #[tokio::test]
    async fn gives_every_match_of_a_line() {
        let r = repo("search-ranges");
        write(&r, "a.txt", "foo bar foo\nnone\n");
        let res = search(&r.to_string_lossy(), &query("foo")).await.unwrap();
        let m = &res.matches[0];
        assert_eq!(
            (m.col, m.offset, m.text.as_str(), m.ranges.clone()),
            (1, 0, "foo bar foo", vec![[0, 3], [8, 11]])
        );
        // Each match of an expression, whole.
        let mut q = query("o+ |r");
        q.regex = true;
        let res = search(&r.to_string_lossy(), &q).await.unwrap();
        assert_eq!(res.matches[0].ranges, vec![[1, 4], [6, 7]]);
    }

    #[tokio::test]
    async fn counts_matches_in_characters_after_accents_and_emoji() {
        let r = repo("search-emoji");
        write(&r, "a.txt", "é😀 foo foo\n");
        let res = search(&r.to_string_lossy(), &query("foo")).await.unwrap();
        let m = &res.matches[0];
        assert_eq!(
            (m.col, m.offset, m.ranges.clone()),
            (4, 0, vec![[3, 6], [7, 10]])
        );
    }

    #[tokio::test]
    async fn shows_a_long_line_around_its_first_match() {
        let r = repo("search-window");
        let far = format!("{}needle{}needle\n", "ü".repeat(400), "x".repeat(400));
        let near = format!("{}needle{}\n", "a".repeat(10), "b".repeat(500));
        let end = format!("{}needle\n", "y".repeat(1000));
        write(&r, "long.txt", &format!("{far}{near}{end}"));
        let res = search(&r.to_string_lossy(), &query("needle"))
            .await
            .unwrap();
        // 40 characters before the first match; the second one, further, is not in the text.
        let m = &res.matches[0];
        assert_eq!(
            (m.col, m.offset, m.ranges.clone()),
            (401, 360, vec![[40, 46]])
        );
        assert_eq!(
            m.text,
            format!("{}needle{}", "ü".repeat(40), "x".repeat(254))
        );
        let m = &res.matches[1];
        assert_eq!((m.col, m.offset, m.ranges.clone()), (11, 0, vec![[10, 16]]));
        assert_eq!(m.text.chars().count(), 300);
        // At the end of the line: the text is still 300 characters long.
        let m = &res.matches[2];
        assert_eq!(
            (m.col, m.offset, m.ranges.clone()),
            (1001, 706, vec![[294, 300]])
        );
    }

    fn regex(pattern: &str) -> SearchQuery {
        SearchQuery {
            regex: true,
            ..query(pattern)
        }
    }

    /// What `pattern` finds in `digits.txt`, `words.txt` and `space.txt`, with `engine`.
    async fn javascript_like(
        r: &Path,
        pattern: &str,
        engine: Engine,
    ) -> Result<Vec<(String, u32)>> {
        let res = run(&r.to_string_lossy(), &regex(pattern), TIME_LIMIT, engine).await?;
        let mut out: Vec<_> = res.matches.into_iter().map(|m| (m.path, m.line)).collect();
        out.sort();
        Ok(out)
    }

    /// A repository of new files only, none of them with digits but `digits.txt`.
    fn javascript_repo(name: &str) -> PathBuf {
        let r = test_dir(name);
        git(&r, &["init", "-q", "-b", "main"]);
        write(&r, "digits.txt", "foods\n42 apples\n");
        write(&r, "words.txt", "foo bar\nfoobar\n");
        write(&r, "space.txt", "a b\nab\n");
        r
    }

    #[tokio::test]
    async fn reads_an_expression_as_javascript_writes_it() {
        let r = javascript_repo("search-js");
        let e = engine().await;
        assert_eq!(
            javascript_like(&r, r"\d+", e).await.unwrap(),
            vec![at("digits.txt", 2)]
        );
        assert_eq!(
            javascript_like(&r, r"\bfoo\b", e).await.unwrap(),
            vec![at("words.txt", 1)]
        );
        assert_eq!(
            javascript_like(&r, r"a\sb", e).await.unwrap(),
            vec![at("space.txt", 1)]
        );
        if e.perl {
            // What only Perl's expressions (and JavaScript's) have: a lookbehind.
            assert_eq!(
                javascript_like(&r, r"(?<=foo )bar", e).await.unwrap(),
                vec![at("words.txt", 1)]
            );
        } else {
            eprintln!("git has no -P here: only the extended expressions are tested");
        }
    }

    #[tokio::test]
    async fn translates_an_expression_for_a_git_without_perl() {
        let r = javascript_repo("search-js-ere");
        let e = Engine {
            perl: false,
            ere: engine().await.ere,
        };
        assert_eq!(
            javascript_like(&r, r"\d+", e).await.unwrap(),
            vec![at("digits.txt", 2)]
        );
        assert_eq!(
            javascript_like(&r, r"a\sb", e).await.unwrap(),
            vec![at("space.txt", 1)]
        );
        let words = javascript_like(&r, r"\bfoo\b", e).await;
        if e.ere == Ere::Posix {
            let err = words.unwrap_err().to_string();
            assert!(err.contains("sur ce poste : \\b"), "{err}");
        } else {
            assert_eq!(words.unwrap(), vec![at("words.txt", 1)]);
        }
    }

    #[test]
    fn turns_the_escapes_of_javascript_into_posix_classes() {
        for ere in [Ere::Gnu, Ere::Bsd, Ere::Posix] {
            assert_eq!(
                to_ere(r"\d+\D\s\S\w\W\.\t", ere).unwrap(),
                "[0-9]+[^0-9][[:space:]][^[:space:]][[:alnum:]_][^[:alnum:]_]\\.\t"
            );
            // In a set of characters, as POSIX writes them: `]` first, `-` last.
            assert_eq!(
                to_ere(r"[\d\s\w.\-\]]", ere).unwrap(),
                "[]0-9[:space:][:alnum:]_.-]"
            );
            assert_eq!(to_ere(r"[^a-z\^]", ere).unwrap(), "[^a-z^]");
            // What is written for git already is left as it is.
            let posix = r"(fn|struct)[[:space:]]+run([^[:alnum:]_]|$)";
            assert_eq!(to_ere(posix, ere).unwrap(), posix);
            // As the editor looks for a definition.
            assert_eq!(
                to_ere(r"(function\*?|class)\s+run(\W|$)", ere).unwrap(),
                r"(function\*?|class)[[:space:]]+run([^[:alnum:]_]|$)"
            );
            let refused = to_ere(r"[\D]", ere).unwrap_err().to_string();
            assert_eq!(
                refused,
                "Cette expression n'est pas prise en charge par git sur ce poste : [\\D]"
            );
        }
        assert_eq!(to_ere(r"\bfoo\b|\Bx", Ere::Gnu).unwrap(), r"\bfoo\b|\Bx");
        assert_eq!(
            to_ere(r"\b(function|class)\s+run\b|\btype\s+\w\b", Ere::Bsd).unwrap(),
            "[[:<:]](function|class)[[:space:]]+run[[:>:]]|[[:<:]]type[[:space:]]+[[:alnum:]_][[:>:]]"
        );
        for (pattern, what) in [(r"(a|b)\b", r"\b"), (r"\Bx", r"\B")] {
            assert_eq!(
                to_ere(pattern, Ere::Bsd).unwrap_err().to_string(),
                format!("Cette expression n'est pas prise en charge par git sur ce poste : {what}")
            );
        }
        assert_eq!(
            to_ere(r"\bfoo", Ere::Posix).unwrap_err().to_string(),
            "Cette expression n'est pas prise en charge par git sur ce poste : \\b"
        );
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

    /// What `parse_line` reads of a line of git's, its marks around `match`: `before match after`.
    fn marked(column: usize, before: &[u8], hit: &[u8], after: &[u8]) -> Vec<u8> {
        let mut raw = format!("a.txt\x001\x00{column}\x00").into_bytes();
        for part in [before, MARK_ON, hit, MARK_OFF, after, b"\n"] {
            raw.extend_from_slice(part);
        }
        raw
    }

    #[test]
    fn decodes_the_line_whole_when_git_marks_a_match_inside_a_character() {
        // A git comparing bytes (no UTF-8 locale) ends `caf([^[:alnum:]_]|$)` in the middle of `é`.
        let m = parse_line(&marked(10, b"function ", b"caf\xc3", b"\xa9bar() {}")).unwrap();
        assert_eq!(
            (m.text.as_str(), m.col, m.ranges.clone()),
            ("function cafébar() {}", 10, vec![[9, 12]])
        );
        // Or begins it there: the match holds the whole character.
        let m = parse_line(&marked(5, b"caf\xc3", b"\xa9bar", b"!")).unwrap();
        assert_eq!(
            (m.text.as_str(), m.col, m.ranges.clone()),
            ("cafébar!", 4, vec![[3, 7]])
        );
    }

    #[tokio::test]
    async fn a_git_comparing_bytes_still_gives_the_line_unbroken() {
        let r = javascript_repo("search-bytes");
        write(&r, "d.txt", "function cafébar() {}\n");
        let q = regex("caf([^[:alnum:]_]|$)");
        let args = grep_args(&q, &q.pattern, false, true);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = git::command(&r.to_string_lossy(), &args)
            .env("LC_ALL", "C")
            .output()
            .await
            .unwrap();
        let lines: Vec<_> = out
            .stdout
            .split_inclusive(|b| *b == b'\n')
            .filter_map(parse_line)
            .collect();
        assert_eq!(lines.len(), 1, "{}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(lines[0].text, "function cafébar() {}");
        assert_eq!(lines[0].ranges, vec![[9, 12]]);
    }

    #[test]
    fn leaves_out_the_marks_of_a_line_that_holds_some_of_its_own() {
        // A colored log: its `ESC[m` inside the match would end it too soon.
        let m = parse_line(&marked(4, b"ab ", b"fo\x1b[mo", b" z")).unwrap();
        assert_eq!((m.col, m.ranges.clone()), (4, vec![]));
        // A mark of its own before the match: git's column is trusted, not the marks.
        let m = parse_line(&marked(12, b"x\x1b[5;7;9my ", b"foo", b"")).unwrap();
        assert_eq!((m.col, m.ranges.clone()), (12, vec![]));
        // Its own marks paired up before the match: they do not begin where git's column says.
        let m = parse_line(&marked(15, b"x\x1b[5;7;9my\x1b[m ", b"foo", b"")).unwrap();
        assert_eq!(m.ranges, Vec::<[u32; 2]>::new());
        // git's own marks alone are taken.
        let m = parse_line(&marked(4, b"ab ", b"foo", b" z")).unwrap();
        assert_eq!((m.col, m.ranges.clone()), (4, vec![[3, 6]]));
    }

    #[test]
    fn makes_a_folder_no_one_can_guess_for_the_probe() {
        let parent = test_dir("search-probe-dir");
        let a = fresh_dir(&parent).unwrap();
        let b = fresh_dir(&parent).unwrap();
        assert_ne!(a, b);
        assert!(a.is_dir() && b.is_dir());
        let name = a.file_name().unwrap().to_string_lossy().to_string();
        assert!(name.starts_with("escouade-grep-probe-"), "{name}");
        assert_ne!(name, format!("escouade-grep-probe-{}", std::process::id()));
        assert!(fresh_dir(&parent.join("missing")).is_err());
    }

    #[tokio::test]
    async fn the_probe_never_writes_in_a_folder_it_did_not_make() {
        let parent = test_dir("search-probe-planted");
        // Another user's, made before: at the name the probe used to take.
        let planted = parent.join(format!("escouade-grep-probe-{}", std::process::id()));
        std::fs::create_dir_all(&planted).unwrap();
        std::fs::write(planted.join("p.txt"), "keep\n").unwrap();
        assert!(probe_in(&parent).await.is_some());
        assert_eq!(
            std::fs::read_to_string(planted.join("p.txt")).unwrap(),
            "keep\n"
        );
        // Its own folder is gone after it.
        assert_eq!(std::fs::read_dir(&parent).unwrap().count(), 1);
        assert_eq!(probe_in(&parent.join("missing")).await, None);
    }

    #[tokio::test]
    async fn keeps_only_a_sure_answer_of_the_probe() {
        let cell = OnceCell::new();
        let sure = Engine {
            perl: true,
            ere: Ere::Gnu,
        };
        assert_eq!(cached(&cell, || async { None }).await, Engine::default());
        // Tried again at the next search.
        assert!(cell.get().is_none());
        assert_eq!(cached(&cell, || async { Some(sure) }).await, sure);
        assert_eq!(cached(&cell, || async { None }).await, sure);
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
        assert_eq!((m.line, m.col, m.offset), (2, 401, 106));
        assert_eq!(m.text, format!("{}needle", "ü".repeat(294)));
    }
}
