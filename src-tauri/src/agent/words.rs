//! What the core reads of Claude Code's words, without state, for what it keeps of the agent: a local command's text
//! (`Item::Replayed`), a subagent's transcript. The plugin has its own: this is the core's.

use serde_json::Value;

pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.peek() {
                Some(&'[') => {
                    chars.next();
                    for n in chars.by_ref() {
                        if n.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                // An OSC (a link, a title): to its BEL or its ST (ESC \).
                Some(&']') => {
                    while let Some(n) = chars.next() {
                        if n == '\u{7}' {
                            break;
                        }
                        if n == '\u{1b}' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// The words of a tool's result block.
pub fn tool_result_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|b| b["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}
