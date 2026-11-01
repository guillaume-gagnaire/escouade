//! The calls of an ACP agent under the names Claude Code gives its tools, by their `kind`: `execute` is `Bash`, `edit`
//! `Edit` (or `Write` for a file added), `delete` `Delete`, `read` `Read`, `search` `Grep` (or `Glob`), `fetch`
//! `WebFetch`; another is shown by its title. A write's diff (`content` of type `diff`) is its result.

use serde_json::{json, Value};

/// A call's id as Escouade keeps it: Cursor's has a line break in it (`call-…-0\nfc_…_0`).
pub fn call_id(id: &str) -> String {
    id.replace(['\n', '\r'], "~")
}

/// `path` made absolute in `cwd`, as an agent may name it relative.
pub fn absolute(path: &str, cwd: &str) -> String {
    let of_windows = |p: &str| {
        let b = p.as_bytes();
        (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'\\' | b'/'))
            || p.starts_with('\\')
    };
    if path.is_empty() || cwd.is_empty() || of_windows(path) || path.starts_with('/') {
        return path.to_string();
    }
    let sep = if of_windows(cwd) { '\\' } else { '/' };
    format!("{}{sep}{path}", cwd.trim_end_matches(['/', '\\']))
}

/// The first diff of a call's `content` (`{type: "diff", path, oldText, newText}`): its file, before (none for a file
/// added) and after.
pub fn diff_of(content: &Value) -> Option<(String, Option<String>, String)> {
    content
        .as_array()?
        .iter()
        .find(|c| c["type"] == "diff")
        .and_then(|c| {
            Some((
                c["path"].as_str()?.to_string(),
                c["oldText"].as_str().map(str::to_string),
                c["newText"].as_str().unwrap_or_default().to_string(),
            ))
        })
}

/// The words of a call's `content` (its `content` blocks of text).
pub fn content_text(content: &Value) -> String {
    content
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| c["content"]["text"].as_str().or(c["text"].as_str()))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The `kind` of ACP a tool of Claude Code's name stands for (`tool_of` the other way).
pub fn kind_of(name: &str) -> &'static str {
    match name {
        "Bash" => "execute",
        "Edit" | "Write" => "edit",
        "Delete" => "delete",
        "Read" => "read",
        "Grep" | "Glob" => "search",
        "WebFetch" => "fetch",
        _ => "other",
    }
}

/// The tool Claude Code would have called for the call `u` (`tool_call`: its `kind`, `title`, `rawInput`,
/// `locations`), and its input as Claude Code's.
pub fn tool_of(u: &Value, cwd: &str) -> (String, Value) {
    let raw = &u["rawInput"];
    let located = u["locations"][0]["path"].as_str();
    let path = raw["path"]
        .as_str()
        .or(raw["filePath"].as_str())
        .or(raw["file_path"].as_str())
        .or(located)
        .map(|p| absolute(p, cwd));
    let title = u["title"].as_str().unwrap_or_default();
    let (name, input) = match u["kind"].as_str() {
        Some("execute") => {
            let command = raw["command"]
                .as_str()
                .map(str::to_string)
                .or_else(|| {
                    raw["command"].as_array().map(|parts| {
                        parts
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                })
                .unwrap_or_else(|| title.trim_matches('`').to_string());
            ("Bash", json!({ "command": command }))
        }
        Some("edit") => ("Edit", json!({ "file_path": path })),
        Some("delete") => ("Delete", json!({ "file_path": path })),
        Some("read") => ("Read", json!({ "file_path": path })),
        Some("search") => {
            let glob = raw["globPattern"].as_str().or(raw["glob"].as_str());
            if glob.is_some() {
                (
                    "Glob",
                    json!({ "pattern": glob, "path": raw["targetDirectory"].as_str().or(raw["path"].as_str()) }),
                )
            } else {
                (
                    "Grep",
                    json!({
                        "pattern": raw["pattern"].as_str().or(raw["query"].as_str()).unwrap_or(title),
                        "path": raw["path"],
                    }),
                )
            }
        }
        Some("fetch") => (
            "WebFetch",
            json!({ "url": raw["url"].as_str().unwrap_or(title) }),
        ),
        _ => {
            let name = if title.is_empty() { "Tool" } else { title };
            return (name.to_string(), raw.clone());
        }
    };
    let mut input = input;
    if let Some(o) = input.as_object_mut() {
        o.retain(|_, v| !v.is_null());
    }
    (name.to_string(), input)
}

/// The lines changed between `old` and `new`, as Claude Code's `structuredPatch` gives them: one hunk around what
/// changed (its first and last lines that differ), with three lines of context.
pub fn line_hunks(old: &str, new: &str) -> Value {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    if a == b {
        return json!([]);
    }
    let head = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let tail = a[head..]
        .iter()
        .rev()
        .zip(b[head..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let context = 3;
    let from = head.saturating_sub(context);
    let mut lines: Vec<Value> = a[from..head]
        .iter()
        .map(|l| json!(format!(" {l}")))
        .collect();
    lines.extend(
        a[head..a.len() - tail]
            .iter()
            .map(|l| json!(format!("-{l}"))),
    );
    lines.extend(
        b[head..b.len() - tail]
            .iter()
            .map(|l| json!(format!("+{l}"))),
    );
    let after = (a.len() - tail + context).min(a.len());
    lines.extend(
        a[a.len() - tail..after]
            .iter()
            .map(|l| json!(format!(" {l}"))),
    );
    let start = |n: usize| if n == 0 { 0 } else { from + 1 };
    json!([{ "oldStart": start(a.len()), "newStart": start(b.len()), "lines": lines }])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_call_takes_claude_codes_name_by_its_kind() {
        let cwd = "/repo";
        assert_eq!(
            tool_of(
                &json!({ "kind": "execute", "rawInput": { "command": "npm test" } }),
                cwd
            ),
            ("Bash".into(), json!({ "command": "npm test" }))
        );
        assert_eq!(
            tool_of(
                &json!({ "kind": "edit", "locations": [{ "path": "a.txt" }] }),
                cwd
            ),
            ("Edit".into(), json!({ "file_path": "/repo/a.txt" }))
        );
        assert_eq!(
            tool_of(
                &json!({ "kind": "other", "title": "lookup", "rawInput": { "q": 1 } }),
                cwd
            ),
            ("lookup".into(), json!({ "q": 1 }))
        );
    }

    #[test]
    fn a_change_makes_one_hunk_with_its_context() {
        let h = line_hunks("a\nb\nc\n", "a\nB\nc\n");
        assert_eq!(h[0]["lines"], json!([" a", "-b", "+B", " c"]));
        assert_eq!(line_hunks("", "x")[0]["oldStart"], 0);
    }
}
