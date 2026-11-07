//! Jira's documents (Atlassian Document Format): a description read as markdown, a comment written as a document.

use serde_json::{json, Value};

/// A Jira document (Atlassian Document Format) as markdown: paragraphs, headings, lists, task
/// lists, code, quotes, tables; what has no text (images) is left out.
pub fn adf_to_markdown(doc: &Value) -> String {
    let mut blocks = Vec::new();
    adf_blocks(doc, "", &mut blocks);
    blocks.join("\n\n")
}

/// The blocks of `node`'s content, each as markdown, the lines of list items under `indent`.
fn adf_blocks(node: &Value, indent: &str, out: &mut Vec<String>) {
    for n in children(node) {
        let block = match kind(n) {
            "paragraph" => inline(n),
            "heading" => {
                let level = n["attrs"]["level"].as_u64().unwrap_or(1).clamp(1, 6) as usize;
                format!("{} {}", "#".repeat(level), inline(n))
            }
            "bulletList" | "orderedList" | "taskList" => list(n, indent),
            "codeBlock" => format!(
                "```{}\n{}\n```",
                n["attrs"]["language"].as_str().unwrap_or(""),
                inline(n)
            ),
            "blockquote" => {
                let mut inner = Vec::new();
                adf_blocks(n, "", &mut inner);
                inner
                    .join("\n\n")
                    .lines()
                    .map(|l| format!("> {l}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            "rule" => "---".into(),
            "table" => children(n)
                .map(|row| {
                    let cells: Vec<String> = children(row)
                        .map(|cell| {
                            let mut inner = Vec::new();
                            adf_blocks(cell, "", &mut inner);
                            inner.join(" ").replace('\n', " ")
                        })
                        .collect();
                    format!("| {} |", cells.join(" | "))
                })
                .collect::<Vec<_>>()
                .join("\n"),
            "mediaSingle" | "mediaGroup" | "media" => String::new(),
            // Panels, expands, layouts…: their own blocks.
            _ => {
                adf_blocks(n, indent, out);
                continue;
            }
        };
        if !block.trim().is_empty() {
            out.push(block);
        }
    }
}

fn children(node: &Value) -> impl Iterator<Item = &Value> {
    node["content"].as_array().into_iter().flatten()
}

fn kind(node: &Value) -> &str {
    node["type"].as_str().unwrap_or("")
}

/// A list's items, one per line, a nested list's under them, indented.
fn list(node: &Value, indent: &str) -> String {
    let ordered = kind(node) == "orderedList";
    let start = node["attrs"]["order"].as_u64().unwrap_or(1);
    let mut lines = Vec::new();
    for (i, item) in children(node).enumerate() {
        let bullet = match (kind(item), item["attrs"]["state"].as_str()) {
            ("taskItem", Some("DONE")) => "- [x] ".to_string(),
            ("taskItem", _) => "- [ ] ".to_string(),
            _ if ordered => format!("{}. ", start + i as u64),
            _ => "- ".to_string(),
        };
        let deeper = format!("{indent}  ");
        let mut text = String::new();
        let mut nested = Vec::new();
        if kind(item) == "taskItem" {
            text = inline(item);
        }
        for part in children(item) {
            match kind(part) {
                "bulletList" | "orderedList" | "taskList" => nested.push(list(part, &deeper)),
                "paragraph" if text.is_empty() => text = inline(part),
                "paragraph" => nested.push(format!("{deeper}{}", inline(part))),
                _ => {}
            }
        }
        lines.push(format!("{indent}{bullet}{text}"));
        lines.extend(nested);
    }
    lines.join("\n")
}

/// The text of a node's inline content, with its bold, code and links.
fn inline(node: &Value) -> String {
    let mut s = String::new();
    for n in children(node) {
        let attrs = &n["attrs"];
        match kind(n) {
            "text" => {
                let mut t = n["text"].as_str().unwrap_or("").to_string();
                for m in n["marks"].as_array().into_iter().flatten() {
                    t = match kind(m) {
                        "strong" => format!("**{t}**"),
                        "code" => format!("`{t}`"),
                        "link" => match m["attrs"]["href"].as_str() {
                            Some(href) => format!("[{t}]({href})"),
                            None => t,
                        },
                        _ => t,
                    };
                }
                s.push_str(&t);
            }
            "hardBreak" => s.push('\n'),
            "mention" | "emoji" | "status" => s.push_str(
                attrs["text"]
                    .as_str()
                    .or(attrs["shortName"].as_str())
                    .unwrap_or(""),
            ),
            "inlineCard" => s.push_str(attrs["url"].as_str().unwrap_or("")),
            _ => s.push_str(&inline(n)),
        }
    }
    s
}

/// A text as a Jira document: a paragraph per block of lines, its lines kept apart.
pub fn markdown_to_adf(text: &str) -> Value {
    let paragraphs: Vec<Value> = text
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut content = Vec::new();
            for (i, line) in p.lines().enumerate() {
                if i > 0 {
                    content.push(json!({ "type": "hardBreak" }));
                }
                if !line.is_empty() {
                    content.push(json!({ "type": "text", "text": line }));
                }
            }
            json!({ "type": "paragraph", "content": content })
        })
        .collect();
    json!({ "type": "doc", "version": 1, "content": paragraphs })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_jira_document_reads_as_markdown() {
        let doc = json!({ "type": "doc", "version": 1, "content": [
            { "type": "heading", "attrs": { "level": 2 }, "content": [{ "type": "text", "text": "Contexte" }] },
            { "type": "paragraph", "content": [
                { "type": "text", "text": "Le token " },
                { "type": "text", "text": "expire", "marks": [{ "type": "strong" }] },
                { "type": "text", "text": " trop tôt" },
                { "type": "hardBreak" },
                { "type": "mention", "attrs": { "text": "@Ada" } },
                { "type": "text", "text": " voir " },
                { "type": "text", "text": "la doc", "marks": [{ "type": "link", "attrs": { "href": "https://x.dev" } }] },
            ]},
            { "type": "bulletList", "content": [
                { "type": "listItem", "content": [
                    { "type": "paragraph", "content": [{ "type": "text", "text": "un" }] },
                    { "type": "orderedList", "content": [
                        { "type": "listItem", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "sous" }] }] }
                    ]}
                ]},
                { "type": "listItem", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "deux" }] }] },
            ]},
            { "type": "taskList", "content": [
                { "type": "taskItem", "attrs": { "state": "DONE" }, "content": [{ "type": "text", "text": "fait" }] },
                { "type": "taskItem", "attrs": { "state": "TODO" }, "content": [{ "type": "text", "text": "à faire" }] },
            ]},
            { "type": "codeBlock", "attrs": { "language": "ts" }, "content": [{ "type": "text", "text": "const a = 1;" }] },
            { "type": "mediaSingle", "content": [{ "type": "media", "attrs": { "id": "x" } }] },
            { "type": "panel", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Note" }] }] },
        ]});
        assert_eq!(
            adf_to_markdown(&doc),
            "## Contexte\n\n\
             Le token **expire** trop tôt\n@Ada voir [la doc](https://x.dev)\n\n\
             - un\n  1. sous\n- deux\n\n\
             - [x] fait\n- [ ] à faire\n\n\
             ```ts\nconst a = 1;\n```\n\n\
             Note"
        );
        assert_eq!(adf_to_markdown(&Value::Null), "");
    }

    #[test]
    fn a_text_becomes_a_jira_document_with_its_lines_kept() {
        let doc = markdown_to_adf("Escouade : prêt\n\n✓ un\n○ deux");
        assert_eq!(
            doc,
            json!({ "type": "doc", "version": 1, "content": [
                { "type": "paragraph", "content": [{ "type": "text", "text": "Escouade : prêt" }] },
                { "type": "paragraph", "content": [
                    { "type": "text", "text": "✓ un" }, { "type": "hardBreak" }, { "type": "text", "text": "○ deux" }
                ]},
            ]})
        );
    }
}
