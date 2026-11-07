//! What Escouade reads of Claude Code's words, without state: a local command's text, a tool's result, a window of its
//! quota, and what the conversation says in the app's language.

use serde_json::{json, Value};

/// `fr` or `en`, in the app's language.
pub fn said<'a>(lang: &str, fr: &'a str, en: &'a str) -> &'a str {
    if lang == "en" {
        en
    } else {
        fr
    }
}

/// `s` without its tags and its ANSI codes.
pub fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    strip_ansi(out.trim())
}

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

/// A window's length, in minutes.
const FIVE_HOURS: u32 = 300;
const WEEK: u32 = 10_080;

/// The windows a `rate_limit_event` tells (`unifiedWindows`, utilization 0-1, resetsAt in seconds): some of the
/// account's, as the core's meters (`QuotaMeter`). A model's own week (Fable's) comes without the model's name: `model`
/// empty, the core names it.
pub fn rate_meters(info: &Value) -> Value {
    let w = &info["unifiedWindows"];
    let meter = |key: &str, minutes: u32, model: Option<&str>| {
        let v = w.get(key).filter(|v| v.is_object())?;
        let mut m = json!({
            "minutes": minutes,
            "pct": v["utilization"].as_f64().unwrap_or(0.0) * 100.0,
            "resetsAt": v["resetsAt"].as_i64().map(|s| s * 1000),
        });
        if let Some(model) = model {
            m["model"] = model.into();
        }
        Some(m)
    };
    Value::Array(
        [
            meter("five_hour", FIVE_HOURS, None),
            meter("seven_day", WEEK, None),
            meter("seven_day_overage_included", WEEK, Some("")),
        ]
        .into_iter()
        .flatten()
        .collect(),
    )
}
