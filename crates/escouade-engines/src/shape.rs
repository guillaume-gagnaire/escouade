//! What a tool call's item keeps of its input and of its result, whatever the harness: texts cut to a length, a
//! command's own output, a file's diff (`agent` builds the items with them, `sidechain` too).

use escouade_base::text::truncate;
use serde_json::{json, Map, Value};

/// A tool's result text, at most.
pub const MAX_TEXT: usize = 8000;
/// A text of a tool's input, at most.
pub const MAX_INPUT_STR: usize = 4000;
/// A subagent's prompt (its « consigne ») is kept longer than another input's text.
pub const MAX_PROMPT: usize = 16_000;
/// The report a subagent hands back, kept with its call.
pub const MAX_REPORT: usize = 16_000;
/// The lines of a file's diff kept with its tool's result.
pub const MAX_PATCH_LINES: usize = 800;

/// The `result` of a tool's item: `text`, the words of its result block, but a command's own
/// output when the structured result (`tool_use_result` in the stream, `toolUseResult` in a
/// transcript) has it, cut to `MAX_TEXT`; for a file it changed, the lines of its diff and its path.
pub fn tool_result(mut text: String, is_error: bool, tur: &Value) -> Value {
    let mut result = json!({ "isError": is_error });
    if let Some(stdout) = tur["stdout"].as_str() {
        text = stdout.to_string();
        if let Some(stderr) = tur["stderr"].as_str().filter(|s| !s.is_empty()) {
            text = format!("{text}\n{stderr}");
        }
    }
    result["text"] = Value::String(truncate(text.trim_end(), MAX_TEXT));
    if let Some(hunks) = tur["structuredPatch"].as_array() {
        let (add, del, patch) = if hunks.is_empty() && tur["type"] == "create" {
            created_file_patch(tur["content"].as_str().unwrap_or(""))
        } else {
            summarize_patch(hunks)
        };
        result["add"] = add.into();
        result["del"] = del.into();
        result["patch"] = patch;
        if let Some(path) = tur["filePath"].as_str() {
            result["filePath"] = path.into();
        }
    }
    result
}

pub fn summarize_patch(hunks: &[Value]) -> (u32, u32, Value) {
    let (mut add, mut del, mut kept_total) = (0u32, 0u32, 0usize);
    let mut out = Vec::new();
    for h in hunks {
        let lines = h["lines"].as_array().map(Vec::as_slice).unwrap_or(&[]);
        for l in lines {
            match l.as_str().unwrap_or("").chars().next() {
                Some('+') => add += 1,
                Some('-') => del += 1,
                _ => {}
            }
        }
        if kept_total < MAX_PATCH_LINES {
            let kept: Vec<Value> = lines
                .iter()
                .take(MAX_PATCH_LINES - kept_total)
                .map(|l| Value::String(truncate(l.as_str().unwrap_or(""), 400)))
                .collect();
            kept_total += kept.len();
            out.push(
                json!({ "oldStart": h["oldStart"], "newStart": h["newStart"], "lines": kept }),
            );
        }
    }
    (add, del, Value::Array(out))
}

pub fn created_file_patch(content: &str) -> (u32, u32, Value) {
    let lines: Vec<&str> = content.lines().collect();
    let kept: Vec<Value> = lines
        .iter()
        .take(MAX_PATCH_LINES)
        .map(|l| Value::String(format!("+{}", truncate(l, 400))))
        .collect();
    (
        lines.len() as u32,
        0,
        json!([{ "oldStart": 0, "newStart": 1, "lines": kept }]),
    )
}

/// A tool call's input, as its item keeps it: each text cut to `MAX_INPUT_STR`, but a subagent's
/// prompt (`Task` / `Agent`), its view's « consigne », to `MAX_PROMPT`.
pub fn item_input(name: &str, input: &Value) -> Value {
    let mut kept = trim_strings(input);
    if matches!(name, "Task" | "Agent") {
        if let Some(prompt) = input["prompt"].as_str() {
            kept["prompt"] = Value::String(truncate(prompt, MAX_PROMPT));
        }
    }
    kept
}

/// Truncates long strings inside tool inputs (file contents, big edits) before they reach the UI.
pub fn trim_strings(v: &Value) -> Value {
    match v {
        Value::String(s) if s.len() > MAX_INPUT_STR => Value::String(truncate(s, MAX_INPUT_STR)),
        Value::Array(a) => Value::Array(a.iter().map(trim_strings).collect()),
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, v)| (k.clone(), trim_strings(v)))
                .collect::<Map<_, _>>(),
        ),
        other => other.clone(),
    }
}
