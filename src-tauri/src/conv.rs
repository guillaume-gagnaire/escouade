//! Normalized conversation of an agent, persisted as a JSONL log of append/patch operations.

use crate::model::ConvOp;
use crate::paths;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

pub struct Conv {
    dir: PathBuf,
    agent_id: String,
    loaded: bool,
    items: Vec<Value>,
    index: HashMap<String, usize>,
    writer: Option<BufWriter<File>>,
}

impl Conv {
    pub fn new(dir: &Path, agent_id: &str) -> Self {
        Self {
            dir: dir.to_path_buf(),
            agent_id: agent_id.to_string(),
            loaded: false,
            items: Vec::new(),
            index: HashMap::new(),
            writer: None,
        }
    }

    fn path(&self) -> PathBuf {
        log_path(&self.dir, &self.agent_id)
    }

    pub fn ensure_loaded(&mut self) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        if let Some(log) = replay(&self.path()) {
            self.take(log);
        }
    }

    /// Takes the conversation as its log was read, and compacts the log when it holds many more
    /// operations than items. Not a log read in part only: rewritten from what was read, it would
    /// lose the rest for good. The operations to come are appended to it all the same.
    fn take(&mut self, log: Replayed) {
        self.loaded = true;
        self.items = log.items;
        self.index = log.index;
        if log.partial {
            log::warn!(
                "conversation of {} read in part only: its log is not compacted",
                self.agent_id
            );
        } else if log.ops > self.items.len() * 2 + 64 {
            self.compact();
        }
    }

    /// Rewrites the log as one append per item.
    fn compact(&mut self) {
        self.writer = None;
        if let Err(e) = paths::write_atomic(&self.path(), appends(&self.items).as_bytes()) {
            log::warn!("conversation compaction failed: {e}");
        }
    }

    fn apply_append(&mut self, item: Value) {
        append_item(&mut self.items, &mut self.index, item);
    }

    fn apply_patch(&mut self, id: &str, patch: &Value) {
        patch_item(&mut self.items, &self.index, id, patch);
    }

    /// Applies an operation in memory and persists it (streaming deltas are memory-only;
    /// the final text is persisted by a later patch).
    pub fn apply(&mut self, op: &ConvOp) {
        self.ensure_loaded();
        match op {
            ConvOp::Append { item } => self.apply_append(item.clone()),
            ConvOp::Patch { id, patch } => self.apply_patch(id, patch),
            ConvOp::Delta { id, text } => {
                if let Some(&i) = self.index.get(id) {
                    let cur = self.items[i]["text"].as_str().unwrap_or("").to_string();
                    self.items[i]["text"] = Value::String(cur + text);
                }
                return;
            }
        }
        self.persist(op);
    }

    fn persist(&mut self, op: &ConvOp) {
        if self.writer.is_none() {
            let _ = std::fs::create_dir_all(&self.dir);
            match OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.path())
            {
                Ok(f) => self.writer = Some(BufWriter::new(f)),
                Err(e) => {
                    log::warn!("cannot open conversation log: {e}");
                    return;
                }
            }
        }
        if let (Some(w), Ok(line)) = (self.writer.as_mut(), serde_json::to_string(op)) {
            let _ = w.write_all(line.as_bytes());
            let _ = w.write_all(b"\n");
            let _ = w.flush();
        }
    }

    pub fn items(&mut self) -> Vec<Value> {
        self.ensure_loaded();
        self.items.clone()
    }

    pub fn contains(&mut self, id: &str) -> bool {
        self.ensure_loaded();
        self.index.contains_key(id)
    }

    #[cfg(test)]
    pub fn get(&mut self, id: &str) -> Option<&Value> {
        self.ensure_loaded();
        self.index.get(id).map(|&i| &self.items[i])
    }

    /// Items a turn left open: streaming text blocks (with their text so far, `Some`) and tools
    /// still running (`None`).
    pub fn open_items(&mut self) -> Vec<(String, Option<String>)> {
        self.ensure_loaded();
        self.items
            .iter()
            .filter_map(|v| {
                let id = v["id"].as_str()?.to_string();
                if v["streaming"] == true {
                    Some((id, Some(v["text"].as_str().unwrap_or("").to_string())))
                } else if v["kind"] == "tool" && v["status"] == "running" {
                    Some((id, None))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn ids_where(&mut self, pred: impl Fn(&Value) -> bool) -> Vec<String> {
        self.ensure_loaded();
        self.items
            .iter()
            .filter(|v| pred(v))
            .filter_map(|v| v["id"].as_str().map(str::to_string))
            .collect()
    }

    pub fn delete_file(&mut self) {
        self.writer = None;
        let _ = std::fs::remove_file(self.path());
    }
}

/// The log of an agent's conversation in the conversations' folder `dir`.
pub fn log_path(dir: &Path, agent_id: &str) -> PathBuf {
    dir.join(format!("{agent_id}.jsonl"))
}

/// `items` as a log: one append each, in order.
fn appends(items: &[Value]) -> String {
    let mut out = String::new();
    for item in items {
        out.push_str(&json!({ "op": "append", "item": item }).to_string());
        out.push('\n');
    }
    out
}

/// Writes the log of a new agent's conversation, which starts with `items` (a copy of another
/// agent's): one append each, the history of how they were written left out.
pub fn write_log(dir: &Path, agent_id: &str, items: &[Value]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    paths::write_atomic(&log_path(dir, agent_id), appends(items).as_bytes())
}

/// What a conversation's log leaves once its operations are applied in order.
pub struct Replayed {
    pub items: Vec<Value>,
    /// Each item's place in `items`, by id.
    pub index: HashMap<String, usize>,
    /// The operations read.
    pub ops: usize,
    /// The reading failed before the end of the log (a disk error, not a damaged line): what
    /// follows is not in `items`.
    pub partial: bool,
}

/// Reads a log back, without writing anything: the search reads the logs of agents that are
/// writing theirs. A line that is no operation (cut short as the app stopped while writing it,
/// damaged, not even UTF-8) is skipped and the reading goes on: the log is compacted from what is
/// read. None when there is no log.
pub fn replay(path: &Path) -> Option<Replayed> {
    let file = File::open(path).ok()?;
    Some(replay_from(BufReader::new(file)))
}

/// `replay` of the log `reader` gives.
fn replay_from(reader: impl BufRead) -> Replayed {
    let mut log = Replayed {
        items: Vec::new(),
        index: HashMap::new(),
        ops: 0,
        partial: false,
    };
    // Lines as bytes: reading them as text would stop at the first one that is not UTF-8.
    for line in reader.split(b'\n') {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                log::warn!("conversation log read in part only: {e}");
                log.partial = true;
                break;
            }
        };
        let Ok(v) = serde_json::from_slice::<Value>(&line) else {
            continue;
        };
        log.ops += 1;
        match v["op"].as_str() {
            Some("append") => append_item(&mut log.items, &mut log.index, v["item"].clone()),
            Some("patch") => {
                if let Some(id) = v["id"].as_str() {
                    patch_item(&mut log.items, &log.index, id, &v["patch"]);
                }
            }
            _ => {}
        }
    }
    log
}

/// An item appended again (same id) replaces the one in place.
fn append_item(items: &mut Vec<Value>, index: &mut HashMap<String, usize>, item: Value) {
    let Some(id) = item["id"].as_str().map(str::to_string) else {
        return;
    };
    if let Some(&i) = index.get(&id) {
        items[i] = item;
    } else {
        index.insert(id, items.len());
        items.push(item);
    }
}

fn patch_item(items: &mut [Value], index: &HashMap<String, usize>, id: &str, patch: &Value) {
    let Some(&i) = index.get(id) else { return };
    if let (Some(target), Some(fields)) = (items[i].as_object_mut(), patch.as_object()) {
        for (k, v) in fields {
            target.insert(k.clone(), v.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::test_dir;

    #[test]
    fn a_new_log_holds_the_items_given_one_append_each() {
        let dir = test_dir("conv-write-log");
        let items: Vec<Value> = (0..3)
            .map(|i| json!({ "kind": "user", "id": format!("u{i}"), "text": format!("m{i}") }))
            .collect();
        // Its folder is made when there is none yet.
        let conversations = dir.join("conversations");
        write_log(&conversations, "copy", &items).unwrap();
        let log = replay(&log_path(&conversations, "copy")).unwrap();
        assert_eq!(log.items, items);
        assert_eq!(log.ops, 3);
        assert_eq!(Conv::new(&conversations, "copy").items(), items);
    }

    #[test]
    fn reads_on_past_a_line_that_is_not_utf8_and_compacts_without_losing_what_follows() {
        let dir = test_dir("conv-utf8");
        let op = |v: Value| format!("{v}\n").into_bytes();
        let mut bytes = op(
            json!({ "op": "append", "item": { "kind": "text", "id": "m1", "text": "", "streaming": true } }),
        );
        // Enough operations for the log to be compacted once read.
        for i in 0..70 {
            bytes.extend(op(
                json!({ "op": "patch", "id": "m1", "patch": { "text": format!("v{i}") } }),
            ));
        }
        bytes
            .extend_from_slice(b"{\"op\":\"patch\",\"id\":\"m1\",\"patch\":{\"text\":\"\xff\"}}\n");
        bytes.extend(op(
            json!({ "op": "append", "item": { "kind": "user", "id": "u2", "text": "après" } }),
        ));
        std::fs::write(log_path(&dir, "a1"), bytes).unwrap();
        let ids = |items: &[Value]| -> Vec<String> {
            items
                .iter()
                .map(|i| i["id"].as_str().unwrap().to_string())
                .collect()
        };
        let mut conv = Conv::new(&dir, "a1");
        assert_eq!(ids(&conv.items()), ["m1", "u2"]);
        assert_eq!(conv.get("m1").unwrap()["text"], "v69");
        // Compacted: one append per item, the one after the bad line included.
        let log = replay(&log_path(&dir, "a1")).unwrap();
        assert_eq!(log.ops, 2);
        assert_eq!(ids(&log.items), ["m1", "u2"]);
    }

    /// Gives `bytes` up to `fail_at`, then fails as a disk can: not the end of the file.
    struct FailsMidway {
        bytes: Vec<u8>,
        at: usize,
        fail_at: usize,
    }

    impl std::io::Read for FailsMidway {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.at >= self.fail_at {
                return Err(std::io::Error::other("the disk went away"));
            }
            let n = buf.len().min(self.fail_at - self.at);
            buf[..n].copy_from_slice(&self.bytes[self.at..self.at + n]);
            self.at += n;
            Ok(n)
        }
    }

    #[test]
    fn a_log_whose_reading_fails_midway_is_not_rewritten_from_the_part_read() {
        let dir = test_dir("conv-read-fails-midway");
        let op = |v: Value| format!("{v}\n").into_bytes();
        let mut bytes =
            op(json!({ "op": "append", "item": { "kind": "user", "id": "u1", "text": "avant" } }));
        // Enough operations for the log to be compacted once read.
        for i in 0..70 {
            bytes.extend(op(
                json!({ "op": "patch", "id": "u1", "patch": { "text": format!("v{i}") } }),
            ));
        }
        // The reading fails in the middle of the next line.
        let fail_at = bytes.len() + 10;
        bytes.extend(op(
            json!({ "op": "append", "item": { "kind": "user", "id": "u2", "text": "après" } }),
        ));
        let path = log_path(&dir, "a1");
        std::fs::write(&path, &bytes).unwrap();

        let log = replay_from(BufReader::new(FailsMidway {
            bytes: bytes.clone(),
            at: 0,
            fail_at,
        }));
        assert!(log.partial);
        assert_eq!(log.ops, 71);
        let mut conv = Conv::new(&dir, "a1");
        conv.take(log);
        // What was read is shown, and the log keeps all it holds.
        assert_eq!(conv.items().len(), 1);
        assert_eq!(conv.get("u1").unwrap()["text"], "v69");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            String::from_utf8(bytes).unwrap()
        );

        // Read whole, the same log is compacted.
        assert!(!replay(&path).unwrap().partial);
        assert_eq!(Conv::new(&dir, "a1").items().len(), 2);
        assert_eq!(replay(&path).unwrap().ops, 2);
    }
}
