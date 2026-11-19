//! A plugin in Rust on both targets (docs/PLUGINS-DESIGN.md §8.3): in process natively, a WASM reactor on `wasm32-wasip1`, with
//! the same source.
//!
//! A reactor keeps its state between two calls of its export `escouade-turn`, as an instance of `run.js` does. The host
//! writes on its standard input, which never blocks, then calls a turn: the plugin reads what came and writes its lines,
//! and the turn ends. Nothing waits inside a turn: an answer of the host comes as a line of a later turn.
//!
//! - [`run`] is what a state machine calls in place of [`crate::serve`]; `escouade-plugin-kit` has its own, for a plugin
//!   whose calls wait.
//! - [`crate::module!`] exports the turn. It is one line of the plugin, nothing at all natively.
//! - [`instance_id`] and [`read_file`] are what `std::process::id` and `std::fs` cannot be in a module.
//! - On `wasm32`, `lines`, `write`, `flush`, `set_turn` are the kit's way in, and `host_call` the synchronous
//!   door to the host (`escouade.call`).

use crate::Refusal;
use serde_json::Value;
use std::path::Path;

/// The whole lines of `all` (what was kept of the last turn, then what was read), the end of a line that is not there
/// yet left in it for the next turn. A single line, what a big message is (a dictation of 10 minutes is a line of 25 MB),
/// becomes its `String` in place, with no copy: a module has its memory counted, and a copy of each big line made it
/// fail at random, by where its allocator stood (ESC-528).
#[cfg_attr(not(target_family = "wasm"), allow(dead_code))]
fn whole_lines(all: &mut Vec<u8>) -> Vec<String> {
    let Some(end) = all.iter().rposition(|b| *b == b'\n').map(|i| i + 1) else {
        return Vec::new();
    };
    let rest = all[end..].to_vec();
    let mut whole = std::mem::replace(all, rest);
    whole.truncate(end);
    if whole[..end - 1].contains(&b'\n') {
        return String::from_utf8_lossy(&whole)
            .lines()
            .map(str::to_string)
            .collect();
    }
    // One line: its `\n` (and a `\r` before it) off, as `str::lines` does.
    whole.pop();
    if whole.last() == Some(&b'\r') {
        whole.pop();
    }
    vec![String::from_utf8(whole)
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())]
}

/// Exports the turn of a plugin, `run` being what it runs at its start: `escouade_plugin_protocol::module!(run);`. A
/// module's first turn calls it; it keeps the plugin (`turn::run`, `escouade_plugin_kit::serve::run`), and every turn
/// then runs on what came. Natively, it is nothing: a test calls `run`.
#[macro_export]
macro_rules! module {
    ($run:path) => {
        #[cfg(target_family = "wasm")]
        #[export_name = "escouade-turn"]
        pub extern "C" fn __escouade_turn() {
            $crate::turn::enter($run);
        }
    };
}

/// The loop of a state machine, whatever the target: natively, [`crate::serve`] on the standard input and output until
/// they end, its exit code answered; in a module, `handler` kept for the turns to come.
#[cfg(not(target_family = "wasm"))]
pub fn run(
    name: &str,
    capabilities: Value,
    handler: impl FnMut(&str, Value) -> Result<Value, Refusal> + 'static,
) -> i32 {
    let stdin = std::io::stdin();
    match crate::serve(
        stdin.lock(),
        std::io::stdout().lock(),
        capabilities,
        handler,
    ) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{name}: {e}");
            1
        }
    }
}

/// What tells the ids of this instance from those of the one before it, restarted: its process.
#[cfg(not(target_family = "wasm"))]
pub fn instance_id() -> u64 {
    u64::from(std::process::id())
}

/// A file a state machine must have before it answers: from the disk natively; in a module, read by the host, under a
/// folder the plugin declares (`fs/read`, docs/PLUGINS-DESIGN.md §8.4.6). None when it cannot be read, or is refused.
#[cfg(not(target_family = "wasm"))]
pub fn read_file(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

#[cfg(target_family = "wasm")]
pub use module::*;

#[cfg(target_family = "wasm")]
mod module {
    use super::*;
    use crate::serve;
    use serde_json::json;
    use std::cell::{Cell, RefCell};
    use std::io::{Read, Write};

    type Handler = Box<dyn FnMut(&str, Value) -> Result<Value, Refusal>>;

    struct Machine {
        capabilities: Value,
        handler: Handler,
    }

    thread_local! {
        static STARTED: Cell<bool> = const { Cell::new(false) };
        static TURN: Cell<Option<fn()>> = const { Cell::new(None) };
        static PARTIAL: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
        static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
        static MACHINE: RefCell<Option<Machine>> = const { RefCell::new(None) };
    }

    /// The whole lines the host wrote since the last turn. An empty read is "nothing more for now", never an end.
    pub fn lines() -> Vec<String> {
        PARTIAL.with(|p| {
            // Read after what was kept, in the same buffer: one copy of what came, not three.
            let mut all = p.borrow_mut();
            let mut buf = [0u8; 16 * 1024];
            let mut input = std::io::stdin().lock();
            while let Ok(n) = input.read(&mut buf) {
                if n == 0 {
                    break;
                }
                all.extend_from_slice(&buf[..n]);
            }
            whole_lines(&mut all)
        })
    }

    /// A line for the host, at the end of the turn.
    pub fn write(line: &str) {
        OUT.with(|o| {
            let mut o = o.borrow_mut();
            // The line and its end at once: a push past an exact capacity would double a big line (ESC-528).
            o.reserve(line.len() + 1);
            o.extend_from_slice(line.as_bytes());
            o.push(b'\n');
        });
    }

    /// What the turn wrote goes out, in one write.
    pub fn flush() {
        OUT.with(|o| {
            let mut o = o.borrow_mut();
            if !o.is_empty() {
                let mut out = std::io::stdout().lock();
                let _ = out.write_all(&o);
                let _ = out.flush();
                // A big turn does not keep its memory for the next ones.
                if o.capacity() > 1 << 20 {
                    *o = Vec::new();
                } else {
                    o.clear();
                }
            }
        });
    }

    /// What each turn runs: given once, by what keeps the plugin.
    pub fn set_turn(turn: fn()) {
        TURN.with(|t| t.set(Some(turn)));
    }

    /// One call of `escouade-turn`: `start` the first time, then the turn it set.
    #[doc(hidden)]
    pub fn enter(start: impl FnOnce() -> i32) {
        if !STARTED.with(|s| s.replace(true)) {
            start();
        }
        if let Some(turn) = TURN.with(Cell::get) {
            turn();
        }
    }

    /// See the native `run`: `handler` kept, a turn being [`crate::serve`] on what came.
    pub fn run(
        _name: &str,
        capabilities: Value,
        handler: impl FnMut(&str, Value) -> Result<Value, Refusal> + 'static,
    ) -> i32 {
        let handler = Box::new(handler);
        MACHINE.with(|m| {
            *m.borrow_mut() = Some(Machine {
                capabilities,
                handler,
            })
        });
        set_turn(machine_turn);
        0
    }

    fn machine_turn() {
        let input = lines().join("\n");
        MACHINE.with(|m| {
            let mut m = m.borrow_mut();
            let Some(m) = m.as_mut() else { return };
            let mut out = Vec::new();
            let handler = &mut m.handler;
            let capabilities = m.capabilities.clone();
            let _ = serve(input.as_bytes(), &mut out, capabilities, |m, p| {
                handler(m, p)
            });
            OUT.with(|o| o.borrow_mut().extend_from_slice(&out));
        });
        flush();
    }

    #[link(wasm_import_module = "escouade")]
    extern "C" {
        /// A call served at once on the plugin's own thread: a JSON request, its JSON answer written at `out` (`cap`
        /// bytes): how many bytes it took, or how many it needs as a negative number.
        fn call(request: *const u8, len: usize, out: *mut u8, cap: usize) -> i64;
    }

    #[link(wasm_import_module = "wasi_snapshot_preview1")]
    extern "C" {
        fn random_get(buf: *mut u8, len: usize) -> i32;
    }

    /// A synchronous call of the host (`fs/read`): for what a state machine must have before it answers. Its time counts
    /// in the turn's. A host that asks for more room makes the call again.
    pub fn host_call(method: &str, params: Value) -> Result<Value, String> {
        let request = json!({ "method": method, "params": params }).to_string();
        let mut out = vec![0u8; 64 * 1024];
        loop {
            // SAFETY: both buffers live through the call, which keeps within their lengths.
            let n = unsafe { call(request.as_ptr(), request.len(), out.as_mut_ptr(), out.len()) };
            if n < 0 {
                out.resize(n.unsigned_abs() as usize, 0);
                continue;
            }
            let v: Value = serde_json::from_slice(&out[..n as usize]).map_err(|e| e.to_string())?;
            return match v.get("error") {
                Some(e) => Err(e.as_str().unwrap_or("refused").to_string()),
                None => Ok(v["result"].clone()),
            };
        }
    }

    /// What tells the ids of this instance from those of the one before it, remade: a module has no process, so WASI's
    /// random numbers (`std::process::id` traps here). Of the size of a process's id.
    pub fn instance_id() -> u64 {
        let mut bytes = [0u8; 4];
        // SAFETY: the buffer lives through the call, which fills its length.
        let failed = unsafe { random_get(bytes.as_mut_ptr(), bytes.len()) };
        if failed != 0 {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
            return now.map_or(0, |d| u64::from(d.subsec_nanos()));
        }
        u64::from(u32::from_le_bytes(bytes))
    }

    /// See the native `read_file`.
    pub fn read_file(path: &Path) -> Option<String> {
        let v = host_call("fs/read", json!({ "path": path.to_string_lossy() })).ok()?;
        v["text"].as_str().map(str::to_string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_cut_in_two_comes_whole_at_the_next_turn() {
        let mut all = b"one\ntw".to_vec();
        assert_eq!(whole_lines(&mut all), vec!["one"]);
        assert_eq!(all, b"tw");
        all.extend_from_slice(b"o");
        assert!(whole_lines(&mut all).is_empty());
        all.extend_from_slice(b"\nthree\n");
        assert_eq!(whole_lines(&mut all), ["two", "three"]);
        assert!(all.is_empty());
        // One line, cut as `str::lines` cuts it, a `\r` before its end too; what is not UTF-8, replaced.
        all.extend_from_slice(b"{\"big\":1}\r\n");
        assert_eq!(whole_lines(&mut all), ["{\"big\":1}"]);
        all.extend_from_slice(b"a\xffb\n");
        assert_eq!(whole_lines(&mut all), ["a\u{fffd}b"]);
        all.extend_from_slice(b"\n");
        assert_eq!(whole_lines(&mut all), [""]);
    }

    #[test]
    fn an_instance_has_the_id_of_its_process_natively() {
        assert_eq!(instance_id(), u64::from(std::process::id()));
    }

    #[test]
    fn a_file_is_read_from_the_disk_natively() {
        let this = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        assert!(read_file(&this).is_some_and(|t| t.contains("escouade-plugin-protocol")));
        assert_eq!(read_file(&this.join("nothing")), None);
    }
}
