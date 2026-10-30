//! The sufficiency of an answer (ESC-298): does the index give the agent the code it needs for a
//! task, so that it need not read the code, find that some is missing, and grep again?
//!
//! - For each « tâche » probe, the code an agent needs was written by hand
//!   (`docs/codeindex-probes-sufficiency.json`): ranges `path:from-to`, the function's body and
//!   what it takes to change it without a mistake (the type it takes, a caller, the test that
//!   covers it).
//! - The share of these lines the answer already holds: the lines of its code blocks (those of a
//!   truncated block up to the cut).
//! - The calls it takes to have them all, and their tokens: a range of a symbol whose block was
//!   cut is one `index_body` (when the tool exists), the rest of a file the answer names one Read
//!   of the lines missing, a file it does not name a grep (the informed searches of the probe)
//!   then a Read.
//! - Against grep's path: its searches (blind, then informed when the blind ones found nothing),
//!   then a Read of each range.
//!
//! The answer is read as it was sent: the JSON of the versions before IDX-3 (`result.snippets`),
//! the Markdown of IDX-3 (`#### name (path:line)` and its fenced block).

use super::score::tokens;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

/// The file of the ranges: the commit they were written for, the ranges of each probe.
#[derive(Debug, Deserialize)]
pub struct Needs {
    pub commit: String,
    pub needs: BTreeMap<String, Vec<String>>,
}

/// A range needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Range {
    pub path: String,
    pub from: u32,
    pub to: u32,
}

pub fn parse_range(text: &str) -> Option<Range> {
    let (path, lines) = text.trim().rsplit_once(':')?;
    let (from, to) = match lines.split_once('-') {
        Some((a, b)) => (a.parse().ok()?, b.parse().ok()?),
        None => {
            let n = lines.parse().ok()?;
            (n, n)
        }
    };
    Some(Range {
        path: path.replace('\\', "/"),
        from,
        to,
    })
}

/// A block of code in an answer: its file, its first line, how many lines it shows, and whether
/// it was cut (with the symbol's id when the cut names `index_body`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub path: String,
    pub line: u32,
    pub shown: u32,
    pub cut: bool,
    pub end_line: u32,
}

/// What an answer holds: its blocks, and the files it names.
#[derive(Debug, Default)]
pub struct Held {
    pub blocks: Vec<Block>,
    pub files: BTreeSet<String>,
    /// The symbols it names: path, first and last line (the entry points and the related).
    pub symbols: Vec<(String, u32, u32)>,
}

fn shown_lines(text: &str) -> (u32, bool) {
    let cut = text.contains("\n... [");
    let body = text.split("\n... [").next().unwrap_or("");
    (body.lines().count() as u32, cut)
}

/// Reads an answer: the JSON of `mcp/index.rs` before IDX-3, or the Markdown of IDX-3.
pub fn held(text: &str) -> Held {
    let mut h = Held::default();
    if let Ok(v) = serde_json::from_str::<Value>(text) {
        let r = &v["result"];
        for s in r["snippets"].as_array().into_iter().flatten() {
            let t = s["text"].as_str().unwrap_or_default();
            let (shown, cut) = shown_lines(t);
            h.blocks.push(Block {
                path: s["path"].as_str().unwrap_or_default().to_string(),
                line: s["line"].as_u64().unwrap_or(0) as u32,
                shown,
                cut,
                end_line: s["endLine"].as_u64().unwrap_or(0) as u32,
            });
        }
        for key in ["entryPoints", "related"] {
            for x in r[key].as_array().into_iter().flatten() {
                let path = x["path"].as_str().unwrap_or_default().to_string();
                h.files.insert(path.clone());
                h.symbols.push((
                    path,
                    x["line"].as_u64().unwrap_or(0) as u32,
                    x["endLine"].as_u64().unwrap_or(0) as u32,
                ));
            }
        }
        return h;
    }
    // Markdown: `#### label (path:line)`, then a fenced block.
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        if let Some(rest) = l.strip_prefix("#### ") {
            if let Some((_, loc)) = rest.rsplit_once(" (") {
                let loc = loc.trim_end_matches(')');
                if let Some((path, line)) = loc.rsplit_once(':') {
                    // The fence opens on the next line and closes on a line of three backticks.
                    let start = i + 2;
                    let mut end = start;
                    while end < lines.len() && lines[end] != "```" {
                        end += 1;
                    }
                    let body = lines[start.min(lines.len())..end.min(lines.len())].join("\n");
                    let (shown, cut) = shown_lines(&body);
                    h.blocks.push(Block {
                        path: path.to_string(),
                        line: line.parse().unwrap_or(0),
                        shown,
                        cut,
                        end_line: 0,
                    });
                    h.files.insert(path.to_string());
                    i = end;
                }
            }
        } else if let Some(rest) = l.strip_prefix("- **") {
            // An entry point: `- **name** (kind) - path:line`.
            if let Some((_, loc)) = rest.rsplit_once(" - ") {
                if let Some((path, line)) = loc.rsplit_once(':') {
                    let n = line.parse().unwrap_or(0);
                    h.files.insert(path.to_string());
                    h.symbols.push((path.to_string(), n, n));
                }
            }
        } else if let Some(rest) = l.strip_prefix("- ") {
            // A related file: `- path: name:line, name:line`.
            if let Some((path, list)) = rest.split_once(": ") {
                if !path.contains(' ') && path.contains('.') {
                    h.files.insert(path.to_string());
                    for item in list.split(", ") {
                        if let Some((_, n)) = item.rsplit_once(':') {
                            let n = n.parse().unwrap_or(0);
                            h.symbols.push((path.to_string(), n, n));
                        }
                    }
                }
            }
        }
        i += 1;
    }
    h
}

/// The sufficiency of one answer.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Suffice {
    /// The lines needed, and those the answer holds.
    pub needed: u32,
    pub held: u32,
    /// The calls more to have them all, and their tokens.
    pub more_calls: usize,
    pub more_tokens: u64,
    pub more_body: usize,
    pub more_read: usize,
    pub more_grep: usize,
}

/// How a path to the lines is costed: the lines of a file (`read`), a body by its id (`body`, when
/// the tool exists: the tokens of its lines and its header), the informed grep (`grep`: its
/// calls and tokens).
pub struct Costs<'a> {
    pub dir: &'a Path,
    pub has_body: bool,
    pub grep_calls: usize,
    pub grep_tokens: u64,
}

fn lines_tokens(dir: &Path, path: &str, lines: &[u32]) -> u64 {
    let text = std::fs::read_to_string(dir.join(path)).unwrap_or_default();
    let all: Vec<&str> = text.lines().collect();
    let part: String = lines
        .iter()
        .filter_map(|n| all.get(*n as usize - 1))
        .map(|l| format!("{}\n", l.chars().take(2000).collect::<String>()))
        .collect();
    tokens(&part)
}

/// Measures an answer against the ranges needed.
pub fn measure(text: &str, need: &[Range], c: &Costs) -> Suffice {
    let h = held(text);
    let mut covered: HashMap<&str, BTreeSet<u32>> = HashMap::new();
    for b in &h.blocks {
        let set = covered.entry(b.path.as_str()).or_default();
        for n in b.line..b.line + b.shown {
            set.insert(n);
        }
    }
    let mut s = Suffice::default();
    let mut grepped = false;
    let mut bodies: BTreeSet<(String, u32)> = BTreeSet::new();
    for r in need {
        let missing: Vec<u32> = (r.from..=r.to)
            .filter(|n| {
                !covered
                    .get(r.path.as_str())
                    .is_some_and(|set| set.contains(n))
            })
            .collect();
        s.needed += r.to - r.from + 1;
        s.held += r.to - r.from + 1 - missing.len() as u32;
        if missing.is_empty() {
            continue;
        }
        // A block that was cut, of a symbol holding the range: its body, by `index_body`.
        let cut = h
            .blocks
            .iter()
            .find(|b| b.cut && b.path == r.path && b.line <= r.from && r.from <= b.line + 400);
        if c.has_body {
            if let Some(b) = cut {
                if bodies.insert((b.path.clone(), b.line)) {
                    s.more_body += 1;
                    s.more_calls += 1;
                    let end = if b.end_line > 0 {
                        b.end_line
                    } else {
                        r.to.max(b.line)
                    };
                    let all: Vec<u32> = (b.line..=end).collect();
                    s.more_tokens += lines_tokens(c.dir, &b.path, &all) + 20;
                }
                continue;
            }
        }
        if !h.files.contains(&r.path) && !grepped {
            grepped = true;
            s.more_grep += c.grep_calls;
            s.more_calls += c.grep_calls;
            s.more_tokens += c.grep_tokens;
        }
        s.more_read += 1;
        s.more_calls += 1;
        s.more_tokens += lines_tokens(c.dir, &r.path, &missing);
    }
    s
}

/// grep's path to the same lines: its searches, then a Read of each range.
pub fn grep_path(dir: &Path, need: &[Range], calls: usize, tokens: u64) -> (usize, u64) {
    let read: u64 = need
        .iter()
        .map(|r| lines_tokens(dir, &r.path, &(r.from..=r.to).collect::<Vec<_>>()))
        .sum();
    (calls + need.len(), tokens + read)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_range_is_a_path_and_lines() {
        assert_eq!(
            parse_range("src/a.rs:10-20"),
            Some(Range {
                path: "src/a.rs".into(),
                from: 10,
                to: 20
            })
        );
        assert_eq!(
            parse_range("src/a.rs:7").map(|r| (r.from, r.to)),
            Some((7, 7))
        );
        assert!(parse_range("src/a.rs").is_none());
    }

    #[test]
    fn the_markdown_blocks_are_read_up_to_their_cut() {
        let md = "## Code Context\n### Entry Points\n- **run** (function) - src/a.rs:10\n\n### Related Symbols\n- src/b.rs: helper:3, other:9\n\n### Code\n#### run (src/a.rs:10)\n```rust\nfn run() {\n    a();\n... [truncated — full body: index_body node_id=4]\n```\n\n";
        let h = held(md);
        assert_eq!(
            h.blocks,
            vec![Block {
                path: "src/a.rs".into(),
                line: 10,
                shown: 2,
                cut: true,
                end_line: 0
            }]
        );
        assert!(h.files.contains("src/b.rs") && h.files.contains("src/a.rs"));
    }

    #[test]
    fn the_json_snippets_are_read_too() {
        let v = serde_json::json!({"result": {"entryPoints": [{"path": "src/a.rs", "line": 1, "endLine": 9}],
            "related": [], "snippets": [{"path": "src/a.rs", "line": 1, "endLine": 9, "text": "a\nb\nc"}]}});
        let h = held(&v.to_string());
        assert_eq!(h.blocks[0].shown, 3);
        assert!(!h.blocks[0].cut);
    }

    #[test]
    fn what_is_missing_costs_a_body_a_read_or_a_grep_then_a_read() {
        let dir = crate::paths::test_dir("esc298-suffice");
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/a.rs"), "l1\nl2\nl3\nl4\nl5\nl6\n").unwrap();
        std::fs::write(dir.join("src/c.rs"), "c1\nc2\n").unwrap();
        let md = "### Code\n#### run (src/a.rs:1)\n```rust\nl1\nl2\n... [truncated — full body: index_body node_id=4]\n```\n";
        let need = vec![
            parse_range("src/a.rs:1-4").unwrap(),
            parse_range("src/c.rs:1-2").unwrap(),
        ];
        let costs = |has_body| Costs {
            dir: &dir,
            has_body,
            grep_calls: 2,
            grep_tokens: 100,
        };
        let with = measure(md, &need, &costs(true));
        assert_eq!((with.needed, with.held), (6, 2));
        assert_eq!((with.more_body, with.more_grep, with.more_read), (1, 2, 1));
        assert_eq!(with.more_calls, 4);
        let without = measure(md, &need, &costs(false));
        assert_eq!((without.more_body, without.more_read), (0, 2));
    }
}
