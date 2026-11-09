//! What it asks of the host, in a module: the agent's home by `fs/read` (the root `agentHome`, docs/PLUGINS.md §8.4.6),
//! whether a write reaches a file alone (`fs/fileOnly`, ESC-389), a timer, items said unasked. Natively (the tests) it reads the disk, and
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

    /// The file `rel` of the agent `agent`'s home, by the root `agentHome` alone (its home is the core's to place).
    pub fn read_home(agent: &str, _home: &Path, rel: &Path) -> Option<String> {
        let params = json!({ "agent": agent, "root": "agentHome", "path": rel.to_string_lossy() });
        turn::host_call("fs/read", params).ok()?["text"]
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

    /// Whether a write of `path`, in the folder of the agent `agent`, reaches a file alone (`fs/fileOnly`): a file there,
    /// or nothing and no submodule under it. A host that does not answer (an older core) says no: the user is asked.
    pub fn file_only(agent: &str, _cwd: &str, path: &str) -> bool {
        turn::host_call("fs/fileOnly", json!({ "agent": agent, "path": path }))
            .is_ok_and(|r| r["fileOnly"] == true)
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
pub use module::{file_only, push, read, read_end, read_home, timer};

/// See the module's `read`.
#[cfg(not(target_family = "wasm"))]
pub fn read(_agent: &str, path: &Path) -> Option<String> {
    escouade_plugin_protocol::turn::read_file(path)
}

/// See the module's `read_home`.
#[cfg(not(target_family = "wasm"))]
pub fn read_home(agent: &str, home: &Path, rel: &Path) -> Option<String> {
    read(agent, &home.join(rel))
}

/// See the module's `read_end`.
#[cfg(not(target_family = "wasm"))]
pub fn read_end(agent: &str, path: &Path) -> Option<String> {
    read(agent, path)
}

/// See the module's `file_only`: what the core's `fs/fileOnly` says, on the disk (the tests). A file (not a link), or
/// nothing and no submodule in git's index at it or under it.
#[cfg(not(target_family = "wasm"))]
pub fn file_only(_agent: &str, cwd: &str, path: &str) -> bool {
    let at = Path::new(path);
    match std::fs::symlink_metadata(at) {
        Ok(meta) => meta.file_type().is_file(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let cwd = Path::new(cwd);
            if !cwd.ancestors().any(|d| d.join(".git").exists()) {
                return true;
            }
            let Ok(rel) = at.strip_prefix(cwd) else {
                return false;
            };
            let rel = rel.to_string_lossy().replace('\\', "/");
            std::process::Command::new("git")
                .args(["--literal-pathspecs", "-c", "core.fsmonitor=false"])
                .args(["ls-files", "--stage", "-z", "--", &rel])
                .current_dir(cwd)
                .output()
                .is_ok_and(|out| {
                    out.status.success()
                        && !out
                            .stdout
                            .split(|b| *b == 0)
                            .any(|e| e.starts_with(b"160000 "))
                })
        }
        Err(_) => false,
    }
}

/// See the module's `timer`.
#[cfg(not(target_family = "wasm"))]
pub fn timer(_ms: u64) {}

/// See the module's `push`.
#[cfg(not(target_family = "wasm"))]
pub fn push(_agent: &str, _items: Vec<escouade_plugin_protocol::engine::Item>) {}
