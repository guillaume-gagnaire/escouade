//! What it asks of the host, in a module: the agent's home by `fs/read` (the root `agentHome`, docs/PLUGINS.md §8.4.6), a
//! timer, items said unasked. Natively (the tests, and the core that reads this crate as a library) it reads the disk, and
//! nothing rings: the tests call `Client::tick` themselves.

use std::path::Path;

#[cfg(target_family = "wasm")]
mod module {
    use super::Path;
    use escouade_plugin_protocol::engine::{method, Item, Pushed};
    use escouade_plugin_protocol::turn;
    use serde_json::{json, Value};

    fn read_with(agent: &str, path: &Path, tail: bool) -> Option<Value> {
        let path = path.to_string_lossy();
        let mut params = json!({ "agent": agent, "path": path });
        if tail {
            params["tail"] = json!(true);
        }
        turn::host_call("fs/read", params).ok()
    }

    /// The file `path` of the agent `agent`.
    pub fn read(agent: &str, path: &Path) -> Option<String> {
        read_with(agent, path, false)?["text"]
            .as_str()
            .map(str::to_string)
    }

    /// The end of the file `path` of the agent `agent`, in whole lines: the file itself until it passes what `fs/read` gives
    /// (`limits.fileBytes`). A long conversation's transcript does: what is asked of it is its last steps.
    pub fn read_end(agent: &str, path: &Path) -> Option<String> {
        let read = read_with(agent, path, true)?;
        let text = read["text"].as_str()?;
        Some(match read["cut"] == true {
            true => text
                .split_once('\n')
                .map_or("", |(_, rest)| rest)
                .to_string(),
            false => text.to_string(),
        })
    }

    fn line(method: &str, params: Value) {
        let line = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        turn::write(&line.to_string());
    }

    /// A timer of `ms`: `timer/fired` comes at a later turn. Nothing to answer: which one rang does not matter.
    pub fn timer(ms: u64) {
        line("timer/set", json!({ "ms": ms }));
    }

    /// `items` said of the agent `agent` unasked.
    pub fn push(agent: &str, items: Vec<Item>) {
        let pushed = Pushed {
            agent: agent.to_string(),
            items,
        };
        line(method::ITEMS, json!(pushed));
    }
}

#[cfg(target_family = "wasm")]
pub use module::{push, read, read_end, timer};

/// See the module's `read`.
#[cfg(not(target_family = "wasm"))]
pub fn read(_agent: &str, path: &Path) -> Option<String> {
    escouade_plugin_protocol::turn::read_file(path)
}

/// See the module's `read_end`.
#[cfg(not(target_family = "wasm"))]
pub fn read_end(agent: &str, path: &Path) -> Option<String> {
    read(agent, path)
}

/// See the module's `timer`.
#[cfg(not(target_family = "wasm"))]
pub fn timer(_ms: u64) {}

/// See the module's `push`.
#[cfg(not(target_family = "wasm"))]
pub fn push(_agent: &str, _items: Vec<escouade_plugin_protocol::engine::Item>) {}
