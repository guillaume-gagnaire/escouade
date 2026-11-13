//! « Vue Claude Code »: the conversation of a subagent, read from the transcript of its own that
//! Claude Code writes next to its agent's (`<session>/subagents/agent-<agentId>.jsonl`, with an
//! `agent-<agentId>.meta.json` that names the call which launched it). Read-only: the entries are
//! turned into the items the stream makes of the same blocks (`on_assistant`, `on_user`), for the
//! window to draw as it draws a subagent's thread in the stream view. The window asks for it every
//! 2 s: a file that is as the last read left it (`Cursor`) is not read again. And the files tell
//! what the stream does not, in this view: that the user stopped a subagent in the interface, or
//! that the API gave up on it (`how_it_ended`).

use crate::shape::{item_input, tool_result, MAX_REPORT};
use escouade_base::text::truncate;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

use std::path::{Path, PathBuf};

/// A transcript bigger than that is read by its end.
pub const MAX_FILE: u64 = 8 * 1024 * 1024;
/// The most `*.meta.json` of a folder looked at to find a subagent, and the biggest of them.
const MAX_METAS: usize = 200;
const MAX_META_SIZE: u64 = 64 * 1024;

/// The items of a subagent's thread, from the text of its transcript (`call`: the call that
/// launched it, the `parent` of each item). The forms are the stream's: `text` and `thinking`
/// (never streaming), `tool` (`running` until its result, then `ok` or `error`, with its `result`).
/// Left out: the first `user` entry (its prompt, which its head shows) and any other that is no
/// tool result (an interruption's marker), what Claude Code writes for itself (`attachment`,
/// `api-request*`), an entry of no sidechain, a block with no words (a thinking whose text Claude
/// Code keeps empty), a line that cannot be read.
pub fn sidechain_items(text: &str, call: &str) -> Vec<Value> {
    let mut items: Vec<Value> = Vec::new();
    // Each tool call's place in `items`, for its result to find; every id seen, for none to come
    // twice (a view keys its rows by them).
    let mut tools: HashMap<String, usize> = HashMap::new();
    let mut seen: HashSet<String> = HashSet::new();
    for line in text.lines() {
        let Ok(e) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if e["isSidechain"] != true {
            continue;
        }
        let blocks = e["message"]["content"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default();
        match e["type"].as_str().unwrap_or_default() {
            "assistant" => {
                let uuid = e["uuid"].as_str().unwrap_or_default();
                let ts = e["timestamp"]
                    .as_str()
                    .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
                    .map_or(0, |t| t.timestamp_millis());
                for (rank, block) in blocks.iter().enumerate() {
                    let own = format!("{uuid}:{rank}");
                    let item = match block["type"].as_str().unwrap_or_default() {
                        "text" => json!({
                            "kind": "text", "id": own, "text": block["text"].as_str().unwrap_or_default(),
                            "parent": call, "streaming": false, "ts": ts,
                        }),
                        "thinking" | "redacted_thinking" => json!({
                            "kind": "thinking", "id": own, "text": block["thinking"].as_str().unwrap_or_default(),
                            "parent": call, "streaming": false, "ts": ts,
                        }),
                        "tool_use" | "server_tool_use" => {
                            let name = block["name"].as_str().unwrap_or_default();
                            let mut input = item_input(name, &block["input"]);
                            // The message of its hand-back is its report: kept as long as a report is.
                            if let Some(message) = block["input"]["message"]
                                .as_str()
                                .filter(|_| name == "SubagentHandback")
                            {
                                input["message"] = Value::String(truncate(message, MAX_REPORT));
                            }
                            json!({
                                "kind": "tool", "id": block["id"].as_str().unwrap_or(&own), "name": name,
                                "input": input, "status": "running", "parent": call, "ts": ts,
                            })
                        }
                        _ => continue,
                    };
                    if item["kind"] != "tool"
                        && item["text"].as_str().is_none_or(|t| t.trim().is_empty())
                    {
                        continue;
                    }
                    let id = item["id"].as_str().unwrap_or_default().to_string();
                    if !seen.insert(id.clone()) {
                        continue;
                    }
                    if item["kind"] == "tool" {
                        tools.insert(id, items.len());
                    }
                    items.push(item);
                }
            }
            "user" => {
                let tur = &e["toolUseResult"];
                for b in blocks.iter().filter(|b| b["type"] == "tool_result") {
                    let Some(at) = b["tool_use_id"]
                        .as_str()
                        .and_then(|id| tools.get(id))
                        .copied()
                    else {
                        continue;
                    };
                    let is_error = b["is_error"].as_bool().unwrap_or(false);
                    items[at]["status"] = json!(if is_error { "error" } else { "ok" });
                    items[at]["result"] =
                        tool_result(tool_result_text(&b["content"]), is_error, tur);
                }
            }
            _ => {}
        }
    }
    items
}

/// What an id of Claude Code is made of: the one that names a subagent's file is never a path.
fn is_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// `path` is itself a regular file (or a folder), never a link to one.
fn is_plain(path: &Path, folder: bool) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| if folder { m.is_dir() } else { m.is_file() })
}

/// The transcript of the subagent that `call` launched, among those of the session whose own
/// transcript is `main` (`<folder>/<session>.jsonl`): the file of its `task` when one is told, else
/// the one whose `agent-<id>.meta.json` names the call (`toolUseId`). None when there is none, or
/// when it is a link.
pub fn sidechain_file(main: &Path, call: &str, task: Option<&str>) -> Option<PathBuf> {
    let session = main.parent()?.join(main.file_stem()?);
    let subagents = session.join("subagents");
    if !is_plain(&session, true) || !is_plain(&subagents, true) {
        return None;
    }
    let by_task = task
        .filter(|t| is_id(t))
        .map(|t| subagents.join(format!("agent-{t}.jsonl")))
        .filter(|file| is_plain(file, false));
    by_task.or_else(|| {
        std::fs::read_dir(&subagents)
            .ok()?
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                let id = name.strip_prefix("agent-")?.strip_suffix(".meta.json")?;
                is_id(id).then(|| (entry.path(), id.to_string()))
            })
            .take(MAX_METAS)
            .find_map(|(meta, id)| {
                let named = read_meta(&meta)?;
                let file = subagents.join(format!("agent-{id}.jsonl"));
                (named["toolUseId"] == call && is_plain(&file, false)).then_some(file)
            })
    })
}

/// `file`, found by `sidechain_file` before, is still what it found: an ordinary file in the
/// ordinary folders `<session>/subagents` (a link is never followed, one put there since neither).
pub fn still_found(file: &Path) -> bool {
    let Some(subagents) = file.parent() else {
        return false;
    };
    let Some(session) = subagents.parent() else {
        return false;
    };
    is_plain(file, false) && is_plain(subagents, true) && is_plain(session, true)
}

/// How many paths `Found` keeps.
const MAX_FOUND: usize = 256;

/// Where the transcripts of the subagents were found, by agent and call: finding one means looking
/// through the meta files of the folder, which is not done at each look of the window. Bounded:
/// the oldest goes when it is full. A path that is no longer there is forgotten by its user
/// (`still_found`), the paths of an agent when it is deleted.
#[derive(Default)]
pub struct Found {
    files: Vec<((String, String), PathBuf)>,
}

impl Found {
    #[cfg(any(test, feature = "testing"))]
    // The tests count what is kept; nothing asks whether it is empty.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn has(&self, agent: &str, call: &str) -> bool {
        self.get(agent, call).is_some()
    }

    pub fn get(&self, agent: &str, call: &str) -> Option<PathBuf> {
        self.files
            .iter()
            .find(|((a, c), _)| a == agent && c == call)
            .map(|(_, file)| file.clone())
    }

    pub fn remember(&mut self, agent: &str, call: &str, file: PathBuf) {
        match self
            .files
            .iter_mut()
            .find(|((a, c), _)| a == agent && c == call)
        {
            Some((_, kept)) => *kept = file,
            None => {
                if self.files.len() >= MAX_FOUND {
                    self.files.remove(0);
                }
                self.files
                    .push(((agent.to_string(), call.to_string()), file));
            }
        }
    }

    pub fn forget(&mut self, agent: &str, call: &str) {
        self.files.retain(|((a, c), _)| a != agent || c != call);
    }

    pub fn forget_agent(&mut self, agent: &str) {
        self.files.retain(|((a, _), _)| a != agent);
    }
}

/// The `agent-<id>.meta.json` beside the transcript `file` (`agent-<id>.jsonl`).
pub fn meta_file(file: &Path) -> PathBuf {
    file.with_extension("meta.json")
}

/// What a meta file says: None when it is no ordinary file, is too big, or is no JSON.
pub fn read_meta(path: &Path) -> Option<Value> {
    let small =
        std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file() && m.len() <= MAX_META_SIZE);
    if !small {
        return None;
    }
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// The text of `path`, its last `max` bytes at most, from the start of a line.
pub fn read_text(path: &Path, max: u64) -> std::io::Result<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path)?;
    let len = f.metadata()?.len();
    // From the byte before the window: a line end there means the window starts on a line.
    let start = len.saturating_sub(max).saturating_sub(1);
    f.seek(SeekFrom::Start(start))?;
    let mut buf = Vec::new();
    f.take(max + 1).read_to_end(&mut buf)?;
    let from = if start > 0 {
        buf.iter()
            .position(|b| *b == b'\n')
            .map_or(buf.len(), |i| i + 1)
    } else {
        0
    };
    Ok(String::from_utf8_lossy(&buf[from..]).into_owned())
}

/// How a subagent ended when no hand-back and no notification says it (Claude Code replays those
/// in the agent's transcript, or not): what its own files show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Ended {
    /// The user stopped it, in Claude Code's interface.
    Stopped,
    /// The API gave up on it.
    Failed,
}

impl Ended {
    /// The `status` of the `task_notification` that would tell the same.
    pub fn status(self) -> &'static str {
        match self {
            Ended::Stopped => "stopped",
            Ended::Failed => "failed",
        }
    }
}

/// The texts Claude Code writes as the user's when a turn is interrupted, in an agent's transcript as in
/// a subagent's.
pub const INTERRUPTED: &[&str] = &[
    "[Request interrupted by user",
    "[Request interrupted by user for tool use",
];

/// A user entry whose text is the marker of an interruption (`INTERRUPTED`).
fn interrupts(content: &Value) -> bool {
    let says = |t: &str| INTERRUPTED.iter().any(|m| t.starts_with(m));
    match content.as_str() {
        Some(t) => says(t),
        None => content
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .any(|b| b["type"] == "text" && b["text"].as_str().is_some_and(says)),
    }
}

/// How the subagent ended, from its meta file (`meta`) and its transcript (`text`), if they say:
/// `stoppedByUser`, or a last entry that is the user's « [Request interrupted by user… », is a stop;
/// a last entry that is an API error is a failure. Nothing else says anything: it may run still,
/// or have finished well, which its hand-back or its notification tells. The last entry is the
/// last of the chat (`user`, `assistant`): what Claude Code writes beside it (`attachment`,
/// `api-request*`) does not count, nor a line that cannot be read.
pub fn how_it_ended(meta: Option<&Value>, text: &str) -> Option<Ended> {
    if meta.is_some_and(|m| m["stoppedByUser"] == true) {
        return Some(Ended::Stopped);
    }
    let last = text
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|e| {
            e["isSidechain"] == true && matches!(e["type"].as_str(), Some("user" | "assistant"))
        })?;
    match last["type"].as_str()? {
        "user" if interrupts(&last["message"]["content"]) => Some(Ended::Stopped),
        "assistant" if last["isApiErrorMessage"] == true => Some(Ended::Failed),
        _ => None,
    }
}

/// The model the subagent runs on, as the last of its messages that tells one says
/// (`claude-haiku-5-5`): Claude Code's own (`<synthetic>`) tell nothing.
pub fn model_of(text: &str) -> Option<String> {
    text.lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|e| e["isSidechain"] == true && e["type"] == "assistant")
        .filter_map(|e| e["message"]["model"].as_str().map(str::to_string))
        .find(|m| !m.is_empty() && !m.starts_with('<'))
}

/// How a subagent's files stood when they were last read: the size of its transcript, in bytes,
/// and when it last changed, in ms since the epoch. That is the later of the dates of the
/// transcript and of its meta file, which Claude Code writes on its own (`stoppedByUser`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    pub len: u64,
    pub mtime: i64,
}

/// What a read of a subagent's transcript gives.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SidechainRead {
    pub items: Vec<Value>,
    /// How its files stood: given back by the next read, for it to say they did not change. None
    /// when there is no file yet.
    pub cursor: Option<Cursor>,
    /// How it ended, if its files say (`how_it_ended`).
    pub ended: Option<Ended>,
    /// The model it runs on, once one of its messages told it (`model_of`).
    pub model: Option<String>,
}

/// The cursor of a subagent's files as they stand.
fn stamp(file: &Path) -> std::io::Result<Cursor> {
    let ms = |m: &std::fs::Metadata| {
        m.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_millis().min(i64::MAX as u128) as i64)
    };
    let own = std::fs::metadata(file)?;
    let beside = std::fs::metadata(meta_file(file)).map_or(0, |m| ms(&m));
    Ok(Cursor {
        len: own.len(),
        mtime: ms(&own).max(beside),
    })
}

/// The transcript `file` of the subagent that `call` launched, read again only if it changed since
/// `since` (the cursor of the read before): None when it did not. The cursor is taken before the
/// read, so that what is written meanwhile is read the next time.
pub fn read_changed(
    file: &Path,
    call: &str,
    since: Option<Cursor>,
) -> std::io::Result<Option<SidechainRead>> {
    let cursor = stamp(file)?;
    if since == Some(cursor) {
        return Ok(None);
    }
    let text = read_text(file, MAX_FILE)?;
    let meta = read_meta(&meta_file(file));
    Ok(Some(SidechainRead {
        items: sidechain_items(&text, call),
        cursor: Some(cursor),
        ended: how_it_ended(meta.as_ref(), &text),
        model: model_of(&text),
    }))
}

/// The words of a tool's result block, in a subagent's transcript.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::MAX_TEXT;
    use escouade_model::model::new_id;

    const SIDECHAIN: &str = include_str!("../../../tests/fixtures/real-sidechain.jsonl");
    const META: &str = include_str!("../../../tests/fixtures/real-sidechain.meta.json");
    /// The call that launched it, and its task, as the meta file and the entries say them.
    const CALL: &str = "toolu_01SdqbgNzZvBJ1FgjQC3Gg6L";
    const TASK: &str = "a3c004cb08b483eb9";

    fn dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("escouade-sidechain-{name}-{}", new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A session folder as Claude Code makes it: `<session>.jsonl` and `<session>/subagents/`.
    fn session(name: &str) -> (PathBuf, PathBuf) {
        let root = dir(name);
        let main = root.join("s1.jsonl");
        std::fs::write(&main, "").unwrap();
        let subs = root.join("s1").join("subagents");
        std::fs::create_dir_all(&subs).unwrap();
        (main, subs)
    }

    fn entry(kind: &str, content: Value) -> String {
        let n = new_id();
        json!({ "type": kind, "isSidechain": true, "agentId": TASK, "uuid": n,
                "timestamp": "2026-10-15T11:38:20.516Z", "message": { "content": content } })
        .to_string()
    }

    #[test]
    fn a_real_sidechain_gives_the_thread_of_its_subagent() {
        let items = sidechain_items(SIDECHAIN, CALL);
        let names: Vec<_> = items
            .iter()
            .map(|i| (i["kind"].as_str().unwrap(), i["name"].as_str()))
            .collect();
        // The prompt (shown in the head), the thinking of no words and the interruption are not in
        // it: a write, a command the harness blocked, a word, the same command in the background.
        assert_eq!(
            names,
            [
                ("tool", Some("Write")),
                ("tool", Some("Bash")),
                ("text", None),
                ("tool", Some("Bash")),
            ]
        );
        assert!(items.iter().all(|i| i["parent"] == CALL));
        let write = &items[0];
        assert_eq!(write["id"], "toolu_01SeVcXwq1SFsSvFpMmo3B1C");
        assert_eq!(write["status"], "ok");
        assert_eq!(write["input"]["content"], "x");
        assert_eq!(write["result"]["isError"], false);
        assert!(write["result"]["text"]
            .as_str()
            .unwrap()
            .starts_with("File created successfully"));
        // The time of the entry that called it.
        assert_eq!(write["ts"], 1_792_064_302_918_i64);
        let blocked = &items[1];
        assert_eq!(blocked["status"], "error");
        assert_eq!(blocked["result"]["isError"], true);
        assert!(blocked["result"]["text"]
            .as_str()
            .unwrap()
            .contains("Blocked: sleep 100"));
        assert!(items[2]["text"]
            .as_str()
            .unwrap()
            .starts_with("The harness blocked the foreground"));
        assert_eq!(items[2]["streaming"], false);
        assert_eq!(items[3]["status"], "ok");
        assert_eq!(items[3]["input"]["run_in_background"], true);
    }

    #[test]
    fn what_is_no_part_of_the_thread_is_left_out() {
        let text = [
            entry("user", json!("Fais ceci.")),
            json!({ "type": "attachment", "isSidechain": true, "attachment": { "type": "environment" } })
                .to_string(),
            json!({ "type": "api-request", "timestamp": "2026-10-15T11:38:20.544Z" }).to_string(),
            // An entry of the agent's own chain.
            json!({ "type": "assistant", "isSidechain": false, "uuid": "m0",
                    "message": { "content": [{ "type": "text", "text": "De l’agent." }] } })
            .to_string(),
            "{ not json".to_string(),
            entry("assistant", json!([{ "type": "text", "text": "Du sous-agent." }])),
            entry("assistant", json!([{ "type": "text", "text": "  \n" }])),
            entry("assistant", json!([{ "type": "thinking", "thinking": "" }])),
            entry("user", json!([{ "type": "text", "text": "[Request interrupted by user]" }])),
            entry("user", json!([{ "type": "text", "text": "[Request interrupted by user for tool use]" }])),
        ]
        .join("\n");
        let items = sidechain_items(&text, "c1");
        let said: Vec<_> = items.iter().map(|i| i["text"].as_str().unwrap()).collect();
        assert_eq!(said, ["Du sous-agent."]);
    }

    #[test]
    fn an_item_without_an_id_gets_its_entrys_and_its_rank_and_none_comes_twice() {
        let line = json!({ "type": "assistant", "isSidechain": true, "uuid": "u9",
            "message": { "content": [
                { "type": "thinking", "thinking": "Hmm." },
                { "type": "text", "text": "Voilà." },
                { "type": "tool_use", "name": "Read", "input": { "file_path": "a.rs" } },
                { "type": "tool_use", "id": "k1", "name": "Read", "input": { "file_path": "b.rs" } },
            ] } })
        .to_string();
        // Written twice (a transcript read again, an entry copied): listed once.
        let items = sidechain_items(&format!("{line}\n{line}\n"), "c1");
        let ids: Vec<_> = items.iter().map(|i| i["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["u9:0", "u9:1", "u9:2", "k1"]);
        assert_eq!(items[0]["kind"], "thinking");
        assert_eq!(items[0]["text"], "Hmm.");
    }

    #[test]
    fn a_call_waits_for_its_result_and_a_result_of_no_call_here_is_left_out() {
        let text = [
            entry("assistant", json!([{ "type": "tool_use", "id": "k1", "name": "Bash", "input": { "command": "ls" } }])),
            entry("user", json!([{ "type": "tool_result", "tool_use_id": "elsewhere", "content": "x" }])),
        ]
        .join("\n");
        let items = sidechain_items(&text, "c1");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["status"], "running");
        assert!(items[0].get("result").is_none());
    }

    /// A transcript entry carrying a tool's result, with the structured result beside it.
    fn result_entry(call: &str, content: Value, is_error: bool, structured: Value) -> String {
        let mut e: Value = serde_json::from_str(&entry(
            "user",
            json!([{ "type": "tool_result", "tool_use_id": call, "content": content, "is_error": is_error }]),
        ))
        .unwrap();
        e["toolUseResult"] = structured;
        e.to_string()
    }

    #[test]
    fn a_commands_result_is_its_own_output_cut_to_the_limit() {
        let long = "é".repeat(MAX_TEXT);
        let text = [
            entry("assistant", json!([{ "type": "tool_use", "id": "k1", "name": "Bash", "input": { "command": "ls" } }])),
            result_entry("k1", json!("sans intérêt"), false, json!({ "stdout": long, "stderr": "oups\n" })),
            entry("assistant", json!([{ "type": "tool_use", "id": "k2", "name": "Bash", "input": { "command": "ls" } }])),
            result_entry("k2", json!([{ "type": "text", "text": "a" }, { "type": "text", "text": "b" }]), true, json!("Error: boom")),
        ]
        .join("\n");
        let items = sidechain_items(&text, "c1");
        let out = items[0]["result"]["text"].as_str().unwrap();
        assert!(out.starts_with("éé") && out.ends_with('…'), "cut");
        assert!(out.len() <= MAX_TEXT + '…'.len_utf8());
        assert_eq!(items[0]["status"], "ok");
        // No stdout (an error written as a string): the result block's words.
        assert_eq!(
            items[1]["result"],
            json!({ "isError": true, "text": "a\nb" })
        );
        assert_eq!(items[1]["status"], "error");
    }

    #[test]
    fn the_message_of_its_handback_is_its_report_kept_to_the_length_of_one() {
        let long = "é".repeat(5_000);
        let text = entry(
            "assistant",
            json!([
                { "type": "tool_use", "id": "k1", "name": "SubagentHandback", "input": { "message": long, "other": long } },
                { "type": "tool_use", "id": "k2", "name": "SubagentHandback", "input": { "message": "x".repeat(40_000) } },
            ]),
        );
        let items = sidechain_items(&text, "c1");
        assert_eq!(items[0]["input"]["message"], long, "10 000 bytes: whole");
        assert!(items[0]["input"]["other"].as_str().unwrap().len() < 4_100);
        let kept = items[1]["input"]["message"].as_str().unwrap();
        assert!(
            kept.len() > MAX_REPORT - 10 && kept.len() < MAX_REPORT + 10 && kept.ends_with('…')
        );
    }

    #[test]
    fn an_edit_keeps_its_diff_and_the_file_it_changed() {
        let text = [
            entry("assistant", json!([{ "type": "tool_use", "id": "k1", "name": "Edit", "input": { "file_path": "C:/p/a.ts" } }])),
            result_entry(
                "k1",
                json!("The file has been updated."),
                false,
                json!({ "filePath": "C:/p/a.ts", "structuredPatch": [{ "oldStart": 1, "newStart": 1, "lines": ["-a", "+b", " c"] }] }),
            ),
        ]
        .join("\n");
        let result = &sidechain_items(&text, "c1")[0]["result"];
        assert_eq!((&result["add"], &result["del"]), (&json!(1), &json!(1)));
        assert_eq!(result["filePath"], "C:/p/a.ts");
        assert_eq!(result["patch"][0]["lines"], json!(["-a", "+b", " c"]));
    }

    #[test]
    fn the_file_of_a_task_is_found_next_to_its_session() {
        let (main, subs) = session("by-task");
        let file = subs.join(format!("agent-{TASK}.jsonl"));
        std::fs::write(&file, SIDECHAIN).unwrap();
        assert_eq!(sidechain_file(&main, CALL, Some(TASK)), Some(file.clone()));
        // The task wins: no meta file needed.
        assert_eq!(sidechain_file(&main, "elsewhere", Some(TASK)), Some(file));
        assert_eq!(sidechain_file(&main, CALL, Some("other")), None);
        // A session with nothing of the kind.
        let (bare, _) = session("bare");
        std::fs::remove_dir_all(bare.with_extension("")).unwrap();
        assert_eq!(sidechain_file(&bare, CALL, Some(TASK)), None);
    }

    #[test]
    fn without_a_task_the_meta_file_that_names_the_call_gives_the_file() {
        let (main, subs) = session("by-meta");
        let write = |id: &str, meta: &str| {
            std::fs::write(subs.join(format!("agent-{id}.meta.json")), meta).unwrap();
            std::fs::write(subs.join(format!("agent-{id}.jsonl")), SIDECHAIN).unwrap();
        };
        write("zzz", r#"{"toolUseId":"toolu_other"}"#);
        write("broken", "{ not json");
        write(TASK, META);
        // No file under another's meta: its call finds nothing there.
        std::fs::write(
            subs.join("agent-nofile.meta.json"),
            r#"{"toolUseId":"toolu_nofile"}"#,
        )
        .unwrap();
        // Neither of these is a subagent's.
        std::fs::write(
            subs.join("notes.meta.json"),
            format!(r#"{{"toolUseId":"{CALL}"}}"#),
        )
        .unwrap();
        let expected = subs.join(format!("agent-{TASK}.jsonl"));
        assert_eq!(sidechain_file(&main, CALL, None), Some(expected.clone()));
        // A task of no file falls back on the meta files too.
        assert_eq!(sidechain_file(&main, CALL, Some("late")), Some(expected));
        assert_eq!(sidechain_file(&main, "toolu_nofile", None), None);
        assert_eq!(sidechain_file(&main, "toolu_nobody", None), None);
    }

    #[test]
    fn a_meta_file_over_the_limit_is_not_read() {
        let (main, subs) = session("big-meta");
        // A meta file of `size` bytes, spaces after the JSON.
        let write = |id: &str, call: &str, size: u64| {
            let head = format!(r#"{{"toolUseId":"{call}"}}"#);
            let padding = " ".repeat(size as usize - head.len());
            std::fs::write(subs.join(format!("agent-{id}.meta.json")), head + &padding).unwrap();
            std::fs::write(subs.join(format!("agent-{id}.jsonl")), SIDECHAIN).unwrap();
        };
        write("big", CALL, MAX_META_SIZE + 1);
        write("fits", "toolu_fits", MAX_META_SIZE);
        assert_eq!(sidechain_file(&main, CALL, None), None);
        assert_eq!(
            sidechain_file(&main, "toolu_fits", None),
            Some(subs.join("agent-fits.jsonl"))
        );
    }

    #[test]
    fn a_task_that_is_no_id_is_never_a_path() {
        let (main, subs) = session("task-path");
        // A file a path made of the task could reach.
        std::fs::write(subs.join("agent-a.b.jsonl"), SIDECHAIN).unwrap();
        std::fs::write(subs.join("..").join("outside.jsonl"), SIDECHAIN).unwrap();
        for task in ["a.b", "../outside", "a/b", "a\\b", "", " "] {
            assert_eq!(sidechain_file(&main, CALL, Some(task)), None, "{task:?}");
        }
        // An id is.
        std::fs::write(subs.join("agent-a_b-1.jsonl"), SIDECHAIN).unwrap();
        assert_eq!(
            sidechain_file(&main, CALL, Some("a_b-1")),
            Some(subs.join("agent-a_b-1.jsonl"))
        );
    }

    /// `link` made to point at `target`; false where the system does not let this account do it.
    fn link(target: &Path, link: &Path) -> bool {
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(target, link);
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_file(target, link);
        made.is_ok()
    }

    /// A folder `link` that leads to the folder `target` (a junction under Windows, which needs no
    /// privilege); false where it could not be made.
    fn link_folder(target: &Path, link: &Path) -> bool {
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(target, link).is_ok();
        #[cfg(windows)]
        let made = std::process::Command::new("cmd")
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(link)
            .arg(target)
            .output()
            .is_ok_and(|out| out.status.success());
        made
    }

    #[test]
    fn a_folder_that_is_a_link_is_not_looked_in() {
        let file = format!("agent-{TASK}.jsonl");
        // `subagents` behind a link.
        let (main, subs) = session("subagents-link");
        let real = dir("real-subagents");
        std::fs::write(real.join(&file), SIDECHAIN).unwrap();
        std::fs::remove_dir(&subs).unwrap();
        if !link_folder(&real, &subs) {
            eprintln!("links to folders not allowed here: not tried");
            return;
        }
        assert!(subs.join(&file).exists());
        assert_eq!(sidechain_file(&main, CALL, Some(TASK)), None);
        // The session's folder behind one.
        let root = dir("session-link");
        let main = root.join("s1.jsonl");
        std::fs::write(&main, "").unwrap();
        let real = dir("real-session");
        std::fs::create_dir_all(real.join("subagents")).unwrap();
        std::fs::write(real.join("subagents").join(&file), SIDECHAIN).unwrap();
        assert!(link_folder(&real, &root.join("s1")));
        assert!(root.join("s1").join("subagents").join(&file).exists());
        assert_eq!(sidechain_file(&main, CALL, Some(TASK)), None);
    }

    #[test]
    fn a_link_is_never_followed() {
        let (main, subs) = session("links");
        let elsewhere = dir("elsewhere").join("secret.jsonl");
        std::fs::write(&elsewhere, SIDECHAIN).unwrap();
        // A file that is one is found.
        std::fs::write(subs.join("agent-plain.jsonl"), SIDECHAIN).unwrap();
        assert_eq!(
            sidechain_file(&main, CALL, Some("plain")),
            Some(subs.join("agent-plain.jsonl"))
        );
        // By its task, and by its meta file.
        if !link(&elsewhere, &subs.join(format!("agent-{TASK}.jsonl"))) {
            eprintln!("symbolic links not allowed here: not tried");
            return;
        }
        assert_eq!(sidechain_file(&main, CALL, Some(TASK)), None);
        std::fs::write(subs.join(format!("agent-{TASK}.meta.json")), META).unwrap();
        assert_eq!(sidechain_file(&main, CALL, None), None);
    }

    #[test]
    fn a_big_transcript_is_read_by_its_end_from_the_start_of_a_line() {
        let root = dir("tail");
        let file = root.join("agent-x.jsonl");
        std::fs::write(&file, "{\"n\":1}\n{\"n\":22}\n{\"n\":333}\n").unwrap();
        // All of it when it fits.
        assert_eq!(
            read_text(&file, 100).unwrap(),
            "{\"n\":1}\n{\"n\":22}\n{\"n\":333}\n"
        );
        // A window that starts on a line: that line is whole.
        assert_eq!(read_text(&file, 10).unwrap(), "{\"n\":333}\n");
        assert_eq!(read_text(&file, 19).unwrap(), "{\"n\":22}\n{\"n\":333}\n");
        // One that starts inside a line leaves that line out.
        assert_eq!(read_text(&file, 11).unwrap(), "{\"n\":333}\n");
        assert_eq!(read_text(&file, 20).unwrap(), "{\"n\":22}\n{\"n\":333}\n");
        assert!(read_text(&root.join("nobody.jsonl"), 10).is_err());
    }

    /// The first `n` lines of the real transcript.
    fn first_lines(n: usize) -> String {
        SIDECHAIN.lines().take(n).collect::<Vec<_>>().join("\n") + "\n"
    }

    /// `file` last changed `secs` seconds after the epoch.
    fn touch(file: &Path, secs: u64) {
        std::fs::File::options()
            .write(true)
            .open(file)
            .unwrap()
            .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs))
            .unwrap();
    }

    #[test]
    fn the_model_of_a_subagent_is_the_one_its_last_message_tells() {
        // The real one ran on Haiku, which its call asked for by its alias.
        assert_eq!(model_of(SIDECHAIN).as_deref(), Some("claude-haiku-5-5"));
        let line = |model: &str| {
            json!({"type":"assistant","isSidechain":true,"message":{"model":model,"content":[]}})
                .to_string()
        };
        // Claude Code's own messages tell nothing; an entry of no sidechain is not its.
        let text = [line("claude-sonnet-5-5"), line("<synthetic>")].join(
            "
",
        );
        assert_eq!(model_of(&text).as_deref(), Some("claude-sonnet-5-5"));
        let main = json!({"type":"assistant","message":{"model":"claude-opus-5-5"}}).to_string();
        assert_eq!(model_of(&main), None);
        assert_eq!(model_of(""), None);
    }

    #[test]
    fn a_subagent_stopped_by_the_user_is_told_by_its_meta_file_and_by_the_end_of_its_transcript() {
        let meta: Value = serde_json::from_str(META).unwrap();
        // The real one, which `stop_task` stopped: both say so, and either is enough.
        assert_eq!(how_it_ended(Some(&meta), SIDECHAIN), Some(Ended::Stopped));
        assert_eq!(how_it_ended(None, SIDECHAIN), Some(Ended::Stopped));
        let before = first_lines(10);
        assert_eq!(how_it_ended(Some(&meta), &before), Some(Ended::Stopped));
        // Neither: it may be running still.
        assert_eq!(how_it_ended(None, &before), None);
        for plain in [
            json!({ "toolUseId": CALL }),
            json!({ "stoppedByUser": false }),
        ] {
            assert_eq!(how_it_ended(Some(&plain), &before), None, "{plain}");
        }
        assert_eq!(how_it_ended(None, ""), None);
    }

    #[test]
    fn the_last_entry_that_is_an_interruption_ends_it_whatever_follows_it_that_is_no_part_of_the_chat(
    ) {
        let interrupted = |text: &str| entry("user", json!([{ "type": "text", "text": text }]));
        for text in [
            "[Request interrupted by user]",
            "[Request interrupted by user for tool use]",
        ] {
            assert_eq!(
                how_it_ended(None, &interrupted(text)),
                Some(Ended::Stopped),
                "{text}"
            );
        }
        // The text of an entry may also be a string.
        let as_string = entry("user", json!("[Request interrupted by user]"));
        assert_eq!(how_it_ended(None, &as_string), Some(Ended::Stopped));
        // What Claude Code writes beside the chat, and what cannot be read, is passed over.
        let after = [
            entry("assistant", json!([{ "type": "text", "text": "Je lis." }])),
            interrupted("[Request interrupted by user]"),
            json!({ "type": "attachment", "isSidechain": true }).to_string(),
            json!({ "type": "api-request", "isSidechain": true }).to_string(),
            "{ not json".to_string(),
            String::new(),
        ]
        .join("\n");
        assert_eq!(how_it_ended(None, &after), Some(Ended::Stopped));
        // It went on after (a message took it up again): it is not over.
        let resumed = format!(
            "{after}\n{}",
            entry("assistant", json!([{ "type": "text", "text": "Reprise." }]))
        );
        assert_eq!(how_it_ended(None, &resumed), None);
        // Some other user message, or a result, ends nothing.
        let other = entry("user", json!([{ "type": "text", "text": "Continue." }]));
        assert_eq!(how_it_ended(None, &other), None);
        let result = entry(
            "user",
            json!([{ "type": "tool_result", "tool_use_id": "k1", "content": "[Request interrupted by user]" }]),
        );
        assert_eq!(how_it_ended(None, &result), None);
        // An entry of the agent's own chain is no part of it.
        let agents = json!({ "type": "user", "isSidechain": false,
            "message": { "content": [{ "type": "text", "text": "[Request interrupted by user]" }] } })
        .to_string();
        assert_eq!(how_it_ended(None, &agents), None);
    }

    #[test]
    fn an_error_of_the_api_as_its_last_entry_is_its_failure_and_a_stop_by_the_user_wins() {
        let failed = |content: Value| {
            let mut e: Value = serde_json::from_str(&entry("assistant", content)).unwrap();
            e["isApiErrorMessage"] = json!(true);
            e["error"] = json!("rate_limit");
            e.to_string()
        };
        let error = failed(json!([{ "type": "text", "text": "API Error: 529 overloaded" }]));
        assert_eq!(how_it_ended(None, &error), Some(Ended::Failed));
        // After a normal start, as it comes in a transcript.
        let text = format!("{}\n{error}\n", first_lines(4));
        assert_eq!(how_it_ended(None, &text), Some(Ended::Failed));
        // A message that is no error is no failure; one that came before the last entry neither.
        let said = entry("assistant", json!([{ "type": "text", "text": "Fini." }]));
        assert_eq!(how_it_ended(None, &said), None);
        assert_eq!(how_it_ended(None, &format!("{error}\n{said}")), None);
        // The user's stop is what it was.
        let meta = json!({ "stoppedByUser": true });
        assert_eq!(how_it_ended(Some(&meta), &error), Some(Ended::Stopped));
    }

    #[test]
    fn the_paths_found_are_kept_by_agent_and_call_and_the_oldest_goes_first() {
        let mut found = Found::default();
        assert!(!found.has("a1", "c1"));
        found.remember("a1", "c1", PathBuf::from("x"));
        assert_eq!(found.get("a1", "c1"), Some(PathBuf::from("x")));
        assert!(!found.has("a2", "c1") && !found.has("a1", "c2"));
        // Found again: replaced, not added.
        found.remember("a1", "c1", PathBuf::from("y"));
        assert_eq!(found.len(), 1);
        assert_eq!(found.get("a1", "c1"), Some(PathBuf::from("y")));
        // Only so many: the oldest goes, the latest stays.
        for n in 0..MAX_FOUND + 10 {
            found.remember("a2", &format!("c{n}"), PathBuf::from("f"));
        }
        assert_eq!(found.len(), MAX_FOUND);
        assert!(!found.has("a1", "c1"), "the oldest");
        assert!(!found.has("a2", "c9"));
        assert!(found.has("a2", "c10"));
        assert!(found.has("a2", &format!("c{}", MAX_FOUND + 9)));
        // A path asked again is not pushed out by itself.
        found.remember("a2", "c11", PathBuf::from("g"));
        assert_eq!(found.len(), MAX_FOUND);
        assert!(found.has("a2", "c11") && found.has("a2", "c12"));
        // Forgotten one by one, or all of an agent's.
        found.forget("a2", "c11");
        assert!(!found.has("a2", "c11") && found.has("a2", "c12"));
        found.remember("a3", "c1", PathBuf::from("z"));
        found.forget_agent("a2");
        assert_eq!(found.len(), 1);
        assert!(found.has("a3", "c1"));
    }

    #[test]
    fn a_path_kept_is_good_while_it_and_its_folders_are_plain() {
        let (_, subs) = session("still-found");
        let file = subs.join(format!("agent-{TASK}.jsonl"));
        assert!(!still_found(&file), "no file");
        std::fs::write(&file, SIDECHAIN).unwrap();
        assert!(still_found(&file));
        assert!(!still_found(Path::new("agent.jsonl")));
        // A folder, or a file that went.
        std::fs::remove_file(&file).unwrap();
        std::fs::create_dir(&file).unwrap();
        assert!(!still_found(&file));
        std::fs::remove_dir(&file).unwrap();
        // The folder of the subagents put behind a link since.
        std::fs::write(&file, SIDECHAIN).unwrap();
        let real = dir("still-found-real");
        std::fs::rename(&file, real.join(file.file_name().unwrap())).unwrap();
        std::fs::remove_dir(&subs).unwrap();
        if !link_folder(&real, &subs) {
            eprintln!("links to folders not allowed here: not tried");
            return;
        }
        assert!(subs.join(file.file_name().unwrap()).exists());
        assert!(!still_found(&file));
    }

    #[test]
    fn a_transcript_that_did_not_change_is_not_read_again() {
        let root = dir("unchanged");
        let file = root.join(format!("agent-{TASK}.jsonl"));
        std::fs::write(&file, first_lines(4)).unwrap();
        // Nothing asked before: all of it, and where it stands.
        let first = read_changed(&file, CALL, None).unwrap().unwrap();
        assert_eq!(first.items.len(), 1);
        assert_eq!(first.items[0]["name"], "Write");
        assert_eq!(first.items[0]["status"], "ok");
        assert_eq!(first.ended, None);
        assert_eq!(first.model.as_deref(), Some("claude-haiku-5-5"));
        let cursor = first.cursor.unwrap();
        assert_eq!(cursor.len, std::fs::metadata(&file).unwrap().len());
        // The same size, the same date: nothing.
        assert_eq!(read_changed(&file, CALL, Some(cursor)).unwrap(), None);
        // Another size or another date: read.
        for other in [
            Cursor {
                len: cursor.len + 1,
                ..cursor
            },
            Cursor {
                mtime: cursor.mtime + 1,
                ..cursor
            },
        ] {
            assert_eq!(
                read_changed(&file, CALL, Some(other)).unwrap().unwrap(),
                first
            );
        }
        // No file: an error, as `read_text` has it.
        assert!(read_changed(&root.join("nobody.jsonl"), CALL, None).is_err());
    }

    #[test]
    fn a_transcript_that_grew_gives_its_new_items_and_a_new_cursor() {
        use std::io::Write;
        let root = dir("grown");
        let file = root.join(format!("agent-{TASK}.jsonl"));
        std::fs::write(&file, first_lines(4)).unwrap();
        touch(&file, 1_000);
        let first = read_changed(&file, CALL, None).unwrap().unwrap();
        let before = first.cursor.unwrap();
        assert_eq!(before.mtime, 1_000_000);
        // Appended, as Claude Code writes it.
        let mut f = std::fs::File::options().append(true).open(&file).unwrap();
        f.write_all(
            SIDECHAIN
                .lines()
                .skip(4)
                .take(2)
                .collect::<Vec<_>>()
                .join("\n")
                .as_bytes(),
        )
        .unwrap();
        f.write_all(b"\n").unwrap();
        drop(f);
        touch(&file, 1_005);
        let grown = read_changed(&file, CALL, Some(before)).unwrap().unwrap();
        let names: Vec<_> = grown
            .items
            .iter()
            .map(|i| i["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["Write", "Bash"]);
        let after = grown.cursor.unwrap();
        assert!(after.len > before.len);
        assert_eq!(after.mtime, 1_005_000);
        assert_eq!(grown.ended, None);
        // And the end of it, once the interruption is written.
        let mut f = std::fs::File::options().append(true).open(&file).unwrap();
        f.write_all(SIDECHAIN.lines().last().unwrap().as_bytes())
            .unwrap();
        f.write_all(b"\n").unwrap();
        drop(f);
        let ended = read_changed(&file, CALL, grown.cursor).unwrap().unwrap();
        assert_eq!(ended.ended, Some(Ended::Stopped));
        // Read back as it is.
        assert_eq!(read_changed(&file, CALL, ended.cursor).unwrap(), None);
    }

    #[test]
    fn what_the_meta_file_says_changes_the_cursor_too_and_ends_it() {
        let root = dir("meta-cursor");
        let file = root.join(format!("agent-{TASK}.jsonl"));
        let meta = root.join(format!("agent-{TASK}.meta.json"));
        std::fs::write(&file, first_lines(10)).unwrap();
        touch(&file, 1_000);
        std::fs::write(&meta, r#"{"toolUseId":"x"}"#).unwrap();
        touch(&meta, 900);
        let first = read_changed(&file, CALL, None).unwrap().unwrap();
        assert_eq!(first.ended, None);
        assert_eq!(read_changed(&file, CALL, first.cursor).unwrap(), None);
        // The transcript as it was, the meta file written after: Claude Code says it was stopped.
        std::fs::write(&meta, META).unwrap();
        touch(&meta, 1_100);
        let stopped = read_changed(&file, CALL, first.cursor).unwrap().unwrap();
        assert_eq!(stopped.ended, Some(Ended::Stopped));
        assert_eq!(stopped.cursor.unwrap().mtime, 1_100_000);
        assert_eq!(read_changed(&file, CALL, stopped.cursor).unwrap(), None);
        // A meta file that is too big, or that is no JSON, says nothing.
        std::fs::write(&meta, "{ not json").unwrap();
        assert_eq!(
            read_changed(&file, CALL, None).unwrap().unwrap().ended,
            None
        );
        let big = format!(
            r#"{{"stoppedByUser":true{}}}"#,
            " ".repeat(MAX_META_SIZE as usize)
        );
        std::fs::write(&meta, big).unwrap();
        assert_eq!(
            read_changed(&file, CALL, None).unwrap().unwrap().ended,
            None
        );
    }
}
