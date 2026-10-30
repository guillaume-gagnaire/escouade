//! The tokensave side of the probes bench (ESC-304, IDX-5): each probe of the index asked of
//! tokensave too, by its command line, and its answer scored as the index's is.
//!
//! - Nothing of tokensave is in this repository: the bench runs its program (`--tokensave <exe>`)
//!   with `tool <name> --project <dir> --args <json>`, the same handler its MCP server calls.
//! - The project is a copy without `.git` (`git archive`): the bench refuses a folder inside a git
//!   repository, and runs `tokensave init --no-git-hook` in the copy only when it has no index yet.
//! - Its update check is turned off for each call (`TOKENSAVE_UPDATE_CHECK=off`); its upload
//!   counter is turned off before (`tokensave disable-upload-counter`, `docs/CODEINDEX-PROBES.md`).
//! - The command line gives the whole answer and no metrics line: the answer is cut at 15,000
//!   characters as its MCP server cuts it, and the metrics line it would add is counted with the
//!   index's port of its accounting (`codeindex::accounting`).
//! - The probes it cannot play are « sans objet »: the worktree (one index per folder, no delta)
//!   and the file changed just before the call (the copy is never written).

use super::score::Item;
use super::Probe;
use anyhow::{bail, Context as _, Result};
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// The most characters of an answer of tokensave's MCP server (`truncate_response`).
const MCP_MAX_CHARS: usize = 15_000;

/// tokensave, its program and the copy it indexed.
pub struct Tokensave {
    pub exe: PathBuf,
    pub project: PathBuf,
}

/// An answer: its text as the MCP server would send it, and the time of the call (the process
/// included: the command line opens the index for each call).
pub struct Reply {
    pub text: String,
    pub ms: f64,
    pub failed: bool,
}

impl Tokensave {
    /// Opens the copy, indexing it first (`tokensave init`, timed) when it has no index yet.
    /// Gives the time of the indexing and what it printed.
    pub fn open_or_index(exe: &Path, project: &Path) -> Result<(Self, Option<u128>, String)> {
        no_git(project)?;
        let mut init = (None, String::new());
        if !project.join(".tokensave").is_dir() {
            eprintln!("tokensave init {}…", project.display());
            let t0 = Instant::now();
            let out = Command::new(exe)
                .current_dir(project)
                .env("TOKENSAVE_UPDATE_CHECK", "off")
                .arg("init")
                .arg(project)
                .arg("--no-git-hook")
                .stdin(std::process::Stdio::null())
                .output()
                .with_context(|| format!("running {} init", exe.display()))?;
            let ms = t0.elapsed().as_millis();
            let printed = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            if !out.status.success() {
                bail!("tokensave init failed: {printed}");
            }
            init = (Some(ms), summary(&printed));
        }
        Ok((Self::open(exe, project)?, init.0, init.1))
    }

    /// Checks the copy: no `.git` in it or above it, an index of tokensave in it.
    pub fn open(exe: &Path, project: &Path) -> Result<Self> {
        no_git(project)?;
        if !project.join(".tokensave").is_dir() {
            bail!(
                "no index of tokensave in {}: run `tokensave init` in the copy first",
                project.display()
            );
        }
        let v = Command::new(exe)
            .arg("--version")
            .env("TOKENSAVE_UPDATE_CHECK", "off")
            .output()
            .with_context(|| format!("running {}", exe.display()))?;
        if !v.status.success() {
            bail!("{} --version failed", exe.display());
        }
        Ok(Self {
            exe: exe.to_path_buf(),
            project: project.to_path_buf(),
        })
    }
}

/// What its indexing says, without its colours and its spinner: the lines of the count and of
/// the files skipped.
fn summary(printed: &str) -> String {
    let mut plain = String::new();
    let mut chars = printed.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // An escape: up to its final letter.
            for d in chars.by_ref() {
                if d.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            plain.push(c);
        }
    }
    plain
        .split(['\r', '\n'])
        .filter_map(|l| {
            let l = l.trim();
            let at = l.find("indexing done").or_else(|| l.find("Skipped"))?;
            Some(l[at..].to_string())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// tokensave is never run in a git repository (its hooks, its worktrees): the copy has no `.git`,
/// in it or above it.
fn no_git(project: &Path) -> Result<()> {
    if !project.is_dir() {
        bail!("{} is not a folder", project.display());
    }
    let in_git = Command::new("git")
        .arg("-C")
        .arg(project)
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if in_git || project.join(".git").exists() {
        bail!(
                "{} is in a git repository: tokensave is played only in a copy without .git (git archive)",
                project.display()
            );
    }
    Ok(())
}

impl Tokensave {
    pub fn version(&self) -> String {
        Command::new(&self.exe)
            .arg("--version")
            .env("TOKENSAVE_UPDATE_CHECK", "off")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default()
    }

    /// The size of its index (the folder `.tokensave` of the copy).
    pub fn index_bytes(&self) -> u64 {
        fn size(p: &Path) -> u64 {
            std::fs::read_dir(p)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| match e.metadata() {
                    Ok(m) if m.is_dir() => size(&e.path()),
                    Ok(m) => m.len(),
                    Err(_) => 0,
                })
                .sum()
        }
        size(&self.project.join(".tokensave"))
    }

    /// Calls one of its tools.
    pub fn call(&self, tool: &str, args: &Value) -> Result<Reply> {
        let t0 = Instant::now();
        let out = Command::new(&self.exe)
            .current_dir(&self.project)
            .env("TOKENSAVE_UPDATE_CHECK", "off")
            .arg("tool")
            .arg(tool)
            .arg("--project")
            .arg(&self.project)
            .arg("--args")
            .arg(args.to_string())
            .output()
            .with_context(|| format!("tokensave tool {tool}"))?;
        let ms = (t0.elapsed().as_secs_f64() * 10_000.0).round() / 10.0;
        let failed = !out.status.success();
        // An error is what the agent would read.
        let text = if failed {
            String::from_utf8_lossy(&out.stderr).trim().to_string()
        } else {
            String::from_utf8_lossy(&out.stdout)
                .trim_end_matches(['\r', '\n'])
                .to_string()
        };
        Ok(Reply {
            text: truncate(&text),
            ms,
            failed,
        })
    }
}

/// The answer cut as tokensave's MCP server cuts it.
fn truncate(text: &str) -> String {
    if text.chars().count() <= MCP_MAX_CHARS {
        return text.to_string();
    }
    let cut: String = text.chars().take(MCP_MAX_CHARS).collect();
    format!("{cut}\n\n[... truncated at {MCP_MAX_CHARS} chars]")
}

/// Why a probe is « sans objet » for tokensave, if it is.
pub(super) fn not_played(p: &Probe) -> Option<&'static str> {
    match p.stage.as_str() {
        "worktree" => Some("un index par dossier, pas de delta de worktree"),
        "fresh" | "fresh-gap" => Some("la copie n'est jamais écrite (fraîcheur après écriture)"),
        _ if p.expect.iter().any(|e| unread(e)) => {
            Some("une réponse attendue n'est que dans des .vue, qu'il ne lit pas")
        }
        // A component is its file: tokensave has no symbol for it, nor its uses in a template.
        "base"
            if matches!(p.tool.as_str(), "index_callers" | "index_callees")
                && p.expect.iter().any(|e| e.ends_with(".svelte")) =>
        {
            Some("un composant Svelte n'est pas un symbole chez lui (ni son usage dans un gabarit)")
        }
        _ => None,
    }
}

/// An expected item all of whose places are in files tokensave does not index (`.vue`: « Skipped
/// … unsupported extension » at its indexing).
fn unread(expect: &str) -> bool {
    expect
        .split('|')
        .map(|alt| alt.trim().split(':').next().unwrap_or_default())
        .all(|path| path.ends_with(".vue"))
}

/// The tool of tokensave and its arguments for a probe: the same question, the same limit.
pub(super) fn request(p: &Probe, limit: usize) -> Option<(&'static str, Value)> {
    let a = &p.args;
    let mut m = Map::new();
    let tool = match p.tool.as_str() {
        // No `kind` filter in tokensave_search.
        "index_search" => {
            m.insert("query".into(), json!(a.query.clone()?));
            m.insert("limit".into(), json!(limit));
            "tokensave_search"
        }
        "index_context" => {
            m.insert("task".into(), json!(a.task.clone()?));
            m.insert("max_nodes".into(), json!(limit));
            // Its default is false; the index's (`index_context`) is true: the same for both.
            m.insert("include_code".into(), json!(true));
            "tokensave_context"
        }
        // By the name: tokensave takes no path, it answers each definition apart.
        "index_callers" | "index_callees" => {
            m.insert("symbol".into(), json!(a.name.clone()?));
            m.insert("limit".into(), json!(limit));
            if p.tool == "index_callers" {
                "tokensave_callers"
            } else {
                "tokensave_callees"
            }
        }
        "index_impact" => {
            m.insert("symbol".into(), json!(a.target.clone()?));
            m.insert("max_depth".into(), json!(a.depth.unwrap_or(2).clamp(1, 4)));
            m.insert("limit".into(), json!(limit));
            "tokensave_impact"
        }
        _ => return None,
    };
    Some((tool, Value::Object(m)))
}

/// The context of a task, with keywords or not.
pub fn context_request(task: &str, keywords: &[String], limit: usize) -> Value {
    let mut v = json!({ "task": task, "max_nodes": limit, "include_code": true });
    if !keywords.is_empty() {
        v["keywords"] = json!(keywords);
    }
    v
}

/// A result of tokensave: where it points, and its text in the answer (for the tokens of the
/// wrong ones), with whether its code is shown.
pub struct Hit {
    pub item: Item,
    pub text: String,
    pub shown: bool,
}

/// The results of an answer, in the order the agent reads them: the JSON's objects with a file
/// (callers, callees, impact), the Markdown of the context (entry points, then the related), the
/// lines of the search.
pub fn hits(text: &str) -> Vec<Hit> {
    let trimmed = text.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return dedup(json_hits(text));
    }
    if text.contains("### Entry Points") {
        return dedup(context_hits(text));
    }
    dedup(line_hits(text))
}

fn dedup(v: Vec<Hit>) -> Vec<Hit> {
    let mut out: Vec<Hit> = Vec::new();
    for h in v {
        if !out.iter().any(|o| o.item == h.item) {
            out.push(h);
        }
    }
    out
}

/// The JSON of callers, callees and impact, read as text so that an answer cut at 15,000
/// characters is read up to its cut: each `"file"` is a result, its fields up to the next one. A
/// caller is one result per line where it calls (`lines`), as the index gives them; a symbol
/// given whole (a match, a callee, a node of the impact) is its span, `line` to `end_line`.
fn json_hits(text: &str) -> Vec<Hit> {
    const KEY: &str = "\"file\": \"";
    let starts: Vec<usize> = text.match_indices(KEY).map(|(i, _)| i).collect();
    let mut out = Vec::new();
    for (n, &at) in starts.iter().enumerate() {
        let window = &text[at..starts.get(n + 1).copied().unwrap_or(text.len())];
        let rest = &window[KEY.len()..];
        let Some(q) = rest.find('"') else { continue };
        let path = &rest[..q];
        let num = |key: &str| -> Option<u32> {
            let k = format!("\"{key}\": ");
            let i = window.find(&k)?;
            window[i + k.len()..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .ok()
        };
        let lines: Vec<u32> = window
            .find("\"lines\": [")
            .map(|i| {
                let r = &window[i + 10..];
                r[..r.find(']').unwrap_or(r.len())]
                    .split(',')
                    .filter_map(|x| x.trim().parse().ok())
                    .collect()
            })
            .unwrap_or_default();
        if lines.is_empty() {
            let line = num("line").or_else(|| num("start_line")).unwrap_or(0);
            out.push(Hit {
                item: Item::span(path, line, num("end_line").unwrap_or(line)),
                text: window.to_string(),
                shown: false,
            });
        } else {
            // The text is counted once, with the first line.
            for (k, l) in lines.into_iter().enumerate() {
                out.push(Hit {
                    item: Item::new(path, l),
                    text: if k == 0 {
                        window.to_string()
                    } else {
                        String::new()
                    },
                    shown: false,
                });
            }
        }
    }
    out
}

/// `path:line` in a word.
fn loc(word: &str) -> Option<(String, u32)> {
    // `path:3-9:` in the search's lines.
    let w = word.trim_matches(|c: char| "()[]{},;:`*'\"".contains(c));
    let (path, line) = w.rsplit_once(':')?;
    let line: u32 = line.split('-').next()?.parse().ok()?;
    let path = path.trim_matches(|c: char| "()[]{},;`*'\"".contains(c));
    (path.contains('.') && !path.contains("::") && !path.is_empty())
        .then(|| (path.to_string(), line))
}

/// The context's Markdown: `- **name** (kind) - path:line` for an entry point, `- path: a:1, b:2`
/// for the related (the tests are counted on one line, not listed), the code under
/// `#### name (path:line)`.
fn context_hits(text: &str) -> Vec<Hit> {
    let mut shown: Vec<(String, u32)> = Vec::new();
    let mut in_fence = false;
    for l in text.lines() {
        if l.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some(rest) = l.strip_prefix("#### ") {
            if let Some((_, at)) = rest.rsplit_once(" (") {
                if let Some(x) = loc(at) {
                    shown.push(x);
                }
            }
        }
    }
    let mut out = Vec::new();
    let mut section = "";
    in_fence = false;
    for l in text.lines() {
        if l.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some(s) = l.strip_prefix("### ") {
            section = if s.starts_with("Entry") {
                "entry"
            } else if s.starts_with("Related") {
                "related"
            } else {
                ""
            };
            continue;
        }
        match section {
            "entry" => {
                if let Some(rest) = l.strip_prefix("- **") {
                    if let Some((_, at)) = rest.rsplit_once(" - ") {
                        if let Some((path, line)) = loc(at) {
                            out.push(Hit {
                                shown: shown.iter().any(|(p, n)| *p == path && *n == line),
                                item: Item::new(&path, line),
                                text: l.to_string(),
                            });
                        }
                    }
                }
            }
            "related" => {
                if let Some(rest) = l.strip_prefix("- ") {
                    if let Some((path, list)) = rest.split_once(": ") {
                        if path.contains(' ') || !path.contains('.') {
                            continue;
                        }
                        for x in list.split(", ") {
                            if let Some((_, n)) = x.rsplit_once(':') {
                                if let Ok(n) = n.trim().parse::<u32>() {
                                    out.push(Hit {
                                        item: Item::new(path, n),
                                        text: format!("{x}, "),
                                        shown: false,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// A text answer: each line that names a `path:line`, the first it names.
fn line_hits(text: &str) -> Vec<Hit> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for l in text.lines() {
        if l.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some((path, line)) = l.split_whitespace().find_map(loc) {
            out.push(Hit {
                item: Item::new(&path, line),
                text: l.to_string(),
                shown: false,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_context_markdown_gives_entry_points_then_related_and_skips_the_code() {
        let md = "## Code Context\n**Query:** x\n\n### Entry Points\n- **run** (function) - src/a.rs:10\n  `fn run()`\n- **stop** (method) - src/b.rs:3\n\n### Related Symbols\n- src/c.rs: helper:5, other:9\n- test/fixture files: 4 symbols across 2 files (tokensave_callers on an entry point for details)\n\n### Code\n#### run (src/a.rs:10)\n```rust\nlet x = \"src/z.rs:1\";\n```\n";
        let h = hits(md);
        let got: Vec<(String, u32, bool)> = h
            .iter()
            .map(|h| (h.item.path.clone(), h.item.line, h.shown))
            .collect();
        assert_eq!(
            got,
            vec![
                ("src/a.rs".into(), 10, true),
                ("src/b.rs".into(), 3, false),
                ("src/c.rs".into(), 5, false),
                ("src/c.rs".into(), 9, false),
            ]
        );
    }

    #[test]
    fn a_json_answer_gives_each_place_in_order_up_to_its_cut() {
        let j = "{
  \"matches\": [
    {
      \"qualified_name\": \"f\",
      \"file\": \"src/a.rs\",
      \"line\": 4,
      \"end_line\": 9,
      \"callers\": [
        {
          \"name\": \"g\",
          \"file\": \"src/b.rs\",
          \"line\": 12,
          \"lines\": [
            12,
            15
          ],
          \"def_line\": 10,
          \"end_line\": 20
        },
        {
          \"name\": \"h\",
          \"file\": \"src/c.ts\",
          \"line\": 1,
          \"end_line\": 3
        },
        {
          \"name\": \"cut\",
          \"file\": \"src/d.rs\",
          \"line\": 7

[... truncated at 15000 chars]";
        let h = hits(j);
        let got: Vec<(String, u32, u32)> = h
            .iter()
            .map(|h| (h.item.path.clone(), h.item.line, h.item.end))
            .collect();
        assert_eq!(
            got,
            vec![
                ("src/a.rs".into(), 4, 9),
                ("src/b.rs".into(), 12, 12),
                ("src/b.rs".into(), 15, 15),
                ("src/c.ts".into(), 1, 3),
                ("src/d.rs".into(), 7, 7)
            ]
        );
    }

    #[test]
    fn a_text_answer_gives_the_path_and_line_of_each_line() {
        let t = "src/m.rs:203-205: tokens_of (function) — pub fn tokens_of()\nfn run  src/a.rs:10\n  struct Foo (src/b.rs:3-9)\nno location here\n";
        let h = hits(t);
        let got: Vec<(String, u32)> = h
            .iter()
            .map(|h| (h.item.path.clone(), h.item.line))
            .collect();
        assert_eq!(
            got,
            vec![
                ("src/m.rs".into(), 203),
                ("src/a.rs".into(), 10),
                ("src/b.rs".into(), 3)
            ]
        );
    }

    #[test]
    fn the_indexing_is_summed_up_without_colours_nor_spinner() {
        let out = "\u{1b}[2K\u{1b}[32m✔\u{1b}[0m indexing done — 3 files, 9 nodes in 12ms\n\u{1b}[2K\u{1b}[?25h\n\u{1b}[33mSkipped 2 tracked files (unsupported extension): .vue (2)\u{1b}[0m\nInitialized TokenSave";
        assert_eq!(
            summary(out),
            "indexing done — 3 files, 9 nodes in 12ms\nSkipped 2 tracked files (unsupported extension): .vue (2)"
        );
    }

    #[test]
    fn a_long_answer_is_cut_as_the_mcp_server_cuts_it() {
        let t = "x".repeat(MCP_MAX_CHARS + 10);
        let c = truncate(&t);
        assert!(c.ends_with("[... truncated at 15000 chars]"));
        assert_eq!(truncate("short"), "short");
    }
}
