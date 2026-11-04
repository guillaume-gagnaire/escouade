//! A plugin in Rust as a WASM reactor (ESC-330, docs/PLUGINS.md §8.3): the module keeps its state between two calls of
//! `escouade-turn`, as an instance of `run.js` does. The host writes on its standard input, which never blocks, then
//! calls a turn: the plugin reads what came and writes its lines, and the turn ends. Nothing waits inside a turn: an
//! answer of the host comes as a line of a later turn.

use escouade_plugin_protocol::{serve, Refusal};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::io::{Read, Write};
use std::path::Path;

type Handler = Box<dyn FnMut(&str, Value) -> Result<Value, Refusal>>;

struct Sync {
    capabilities: Value,
    handler: Handler,
}

thread_local! {
    static PARTIAL: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static SYNC: RefCell<Option<Sync>> = const { RefCell::new(None) };
}

/// The whole lines the host wrote since the last turn. An empty read is "nothing more for now", never an end.
pub fn lines() -> Vec<String> {
    let mut buf = [0u8; 16 * 1024];
    let mut all = PARTIAL.with(|p| std::mem::take(&mut *p.borrow_mut()));
    let mut input = std::io::stdin().lock();
    while let Ok(n) = input.read(&mut buf) {
        if n == 0 {
            break;
        }
        all.extend_from_slice(&buf[..n]);
    }
    let end = all.iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
    let rest = all.split_off(end);
    PARTIAL.with(|p| *p.borrow_mut() = rest);
    String::from_utf8_lossy(&all)
        .lines()
        .map(str::to_string)
        .collect()
}

/// A line for the host, at the end of the turn.
pub fn write(line: &str) {
    OUT.with(|o| {
        let mut o = o.borrow_mut();
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
            o.clear();
        }
    });
}

/// A plugin that answers at once, a state machine (the clients of the harnesses): what `run` gives it in place of
/// `escouade_plugin_protocol::serve`, whose loop then runs on what each turn brought.
pub fn host(
    capabilities: Value,
    handler: impl FnMut(&str, Value) -> Result<Value, Refusal> + 'static,
) {
    SYNC.with(|s| {
        *s.borrow_mut() = Some(Sync {
            capabilities,
            handler: Box::new(handler),
        })
    });
}

/// A turn of a plugin that answers at once.
pub fn sync_turn() {
    let input = lines().join("\n");
    SYNC.with(|s| {
        let mut s = s.borrow_mut();
        let Some(s) = s.as_mut() else { return };
        let mut out = Vec::new();
        let handler = &mut s.handler;
        let _ = serve(
            input.as_bytes(),
            &mut out,
            s.capabilities.clone(),
            |m, p| handler(m, p),
        );
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

/// A synchronous call of the host (`fs/read`): for what a state machine must have before it answers.
pub fn host_call(method: &str, params: Value) -> Result<Value, String> {
    let request = json!({ "method": method, "params": params }).to_string();
    let mut out = vec![0u8; 64 * 1024];
    loop {
        // SAFETY: both buffers live through the call, which keeps within their lengths.
        let n = unsafe { call(request.as_ptr(), request.len(), out.as_mut_ptr(), out.len()) };
        if n < 0 {
            out.resize((-n) as usize, 0);
            continue;
        }
        let v: Value = serde_json::from_slice(&out[..n as usize]).map_err(|e| e.to_string())?;
        return match v.get("error") {
            Some(e) => Err(e.as_str().unwrap_or("refused").to_string()),
            None => Ok(v["result"].clone()),
        };
    }
}

/// A file of a folder the plugin declares, read by the host.
pub fn fs_read(path: &Path) -> Option<String> {
    let v = host_call("fs/read", json!({ "path": path.to_string_lossy() })).ok()?;
    v["text"].as_str().map(str::to_string)
}

/// What tells the ids of this instance from those of one before it (a process's id in a native plugin: WASI has none).
pub fn instance_id() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos() as u64 & 0xfff_ffff)
}
