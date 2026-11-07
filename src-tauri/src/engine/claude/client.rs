//! What the core keeps of the words of Claude Code's client, for what stays in it: its models read by a process of their own,
//! the tools a discussion is refused. The agents' own are its plugin's (`plugins/claude/src/client.rs`), which it does not
//! link (docs/PLUGINS.md §8.2).

#[cfg(test)]
use escouade_plugin_protocol::engine::StartAt;
use serde_json::{json, Value};

/// The tools a discussion of the Docs tab is refused: what writes and what runs a command (ESC-261).
pub const WRITES: [&str; 8] = [
    "Edit",
    "Write",
    "MultiEdit",
    "NotebookEdit",
    "Bash",
    "PowerShell",
    "Monitor",
    "EnterWorktree",
];

/// The arguments of a stream of its.
pub const BASE_ARGS: &[&str] = &[
    "--output-format",
    "stream-json",
    "--verbose",
    "--input-format",
    "stream-json",
    "--permission-prompt-tool",
    "stdio",
    "--include-partial-messages",
    // Messages sent from claude.ai (Remote Control) come back on stdout, so the app shows them.
    "--replay-user-messages",
    "--thinking-display",
    "summarized",
    "--allow-dangerously-skip-permissions",
];

/// Whether a model takes an effort, before its account's `claude` has told.
pub fn supports_effort(model: &str) -> bool {
    !model.to_lowercase().contains("haiku")
}

/// The line of a control request.
fn request_line(id: &str, request: Value) -> String {
    json!({ "type": "control_request", "request_id": id, "request": request }).to_string()
}

/// The first request of a process, which asks nothing of the API.
pub fn initialize_line(id: &str) -> String {
    request_line(id, json!({ "subtype": "initialize" }))
}

/// What the answer `frame` to `initialize` of id `id` says: its answer, or why it refused. None for any other line.
pub fn initialized(frame: &Value, id: &str) -> Option<Result<Value, String>> {
    let r = &frame["response"];
    if frame["type"] != "control_response" || r["request_id"] != id {
        return None;
    }
    Some(answer_of(r))
}

fn answer_of(r: &Value) -> Result<Value, String> {
    if r["subtype"] == "success" {
        Ok(r.get("response").cloned().unwrap_or_else(|| json!({})))
    } else {
        Err(r["error"].as_str().unwrap_or("error").to_string())
    }
}

/// What the arguments of a conversation are made of.
#[cfg(test)]
pub struct Conversation<'a> {
    pub model: &'a str,
    pub effort: &'a str,
    pub takes_effort: Option<bool>,
    pub mode: &'a str,
    pub read_only: bool,
    pub append_prompt: Option<&'a str>,
    pub start: &'a StartAt,
}

/// The arguments of a conversation: its model, effort and mode, its added instructions, and where it starts from.
#[cfg(test)]
pub fn conversation_args(c: &Conversation) -> Vec<String> {
    let mut a: Vec<String> = BASE_ARGS.iter().map(|s| s.to_string()).collect();
    a.extend(["--model".into(), c.model.to_string()]);
    if c.takes_effort.unwrap_or_else(|| supports_effort(c.model)) {
        a.extend(["--effort".into(), c.effort.to_string()]);
    }
    a.extend(["--permission-mode".into(), c.mode.to_string()]);
    // A discussion (ESC-261): its tools that write or run a command taken away, whatever its mode becomes (a plan
    // approved puts it in another). In one value, comma separated, as the Agent SDK gives them: what follows is not read
    // as more.
    if c.read_only {
        a.push(format!("--disallowedTools={}", WRITES.join(",")));
    }
    if let Some(p) = c.append_prompt.filter(|p| !p.trim().is_empty()) {
        // On one line: a `.cmd` launcher (npm's claude.cmd) takes no argument with line breaks.
        let one_line = p.split_whitespace().collect::<Vec<_>>().join(" ");
        a.extend(["--append-system-prompt".into(), one_line]);
    }
    match c.start {
        StartAt::New => {}
        StartAt::Resume { session } => a.push(format!("--resume={session}")),
        StartAt::Fork { of, at } => {
            // A copy's conversation so far is its original's, under a new session: the original's is left as it is.
            a.extend([format!("--resume={of}"), "--fork-session".into()]);
            // As it was when copied: the turns the original ran since stay out.
            if let Some(at) = at {
                a.push(format!("--resume-session-at={at}"));
            }
        }
    }
    a
}
