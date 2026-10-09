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
        let Some(log) = replay(&self.path()) else {
            return;
        };
        self.items = log.items;
        self.index = log.index;
        if log.ops > self.items.len() * 2 + 64 {
            self.compact();
        }
    }

    /// Rewrites the log as one append per item.
    fn compact(&mut self) {
        self.writer = None;
        let mut out = String::new();
        for item in &self.items {
            out.push_str(&json!({ "op": "append", "item": item }).to_string());
            out.push('\n');
        }
        if let Err(e) = paths::write_atomic(&self.path(), out.as_bytes()) {
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

/// What a conversation's log leaves once its operations are applied in order.
pub struct Replayed {
    pub items: Vec<Value>,
    /// Each item's place in `items`, by id.
    pub index: HashMap<String, usize>,
    /// The operations read.
    pub ops: usize,
}

/// Reads a log back, without writing anything: the search reads the logs of agents that are
/// writing theirs. A line that is no operation (cut short as the app stopped while writing it,
/// damaged) is skipped. None when there is no log.
pub fn replay(path: &Path) -> Option<Replayed> {
    let file = File::open(path).ok()?;
    let mut log = Replayed {
        items: Vec::new(),
        index: HashMap::new(),
        ops: 0,
    };
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        let Ok(v) = serde_json::from_str::<Value>(&line) else {
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
    Some(log)
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
