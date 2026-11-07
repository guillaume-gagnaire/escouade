//! A plugin in Rust as a reactor (ESC-331, docs/PLUGINS.md §8.3): its state is kept from a turn to the next, it reads what
//! the host wrote, writes its lines, and the turn ends. Each `probe/*` shows the tests one thing of the host. The lines of
//! the host are compact JSON, `id` before `params`: a few searches do, and the module has no dependency.

use std::io::{Read, Write};
use std::sync::Mutex;

#[derive(Default)]
struct Reactor {
    /// How many turns it has run.
    turns: u64,
    /// The first line that ever came.
    first: Option<String>,
    /// What is left of a line that came in two turns.
    partial: Vec<u8>,
    /// The fetches under way: its own id, and the call that waits for it.
    fetches: Vec<(u64, String)>,
    /// Its own ids.
    next: u64,
    /// The lines of its programs.
    heard: Vec<String>,
}

static STATE: Mutex<Option<Reactor>> = Mutex::new(None);

#[link(wasm_import_module = "escouade")]
extern "C" {
    fn call(request: *const u8, len: usize, out: *mut u8, cap: usize) -> i64;
}

/// The state, made by the first turn; the standard error says so, which the journal of the plugin shows.
fn state() -> std::sync::MutexGuard<'static, Option<Reactor>> {
    let mut s = STATE.lock().unwrap();
    if s.is_none() {
        eprintln!("reactor: first turn");
        *s = Some(Reactor::default());
    }
    s
}

/// What follows `"key":` in `line`, whole to its end.
fn after<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let key = format!("\"{key}\":");
    line.find(&key).map(|i| &line[i + key.len()..])
}

/// Its own `id`, not one of its `params`.
fn id_of(line: &str) -> Option<&str> {
    let head = line.split("\"params\":").next().unwrap_or(line);
    let rest = after(head, "id")?;
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    (end > 0).then(|| &rest[..end])
}

fn method_of(line: &str) -> Option<&str> {
    let rest = after(line, "method")?.strip_prefix('"')?;
    rest.find('"').map(|end| &rest[..end])
}

/// The value of `params`, the last key of a line of the host.
fn params_of(line: &str) -> &str {
    after(line, "params").map_or("null", |rest| rest.strip_suffix('}').unwrap_or(rest))
}

/// The value of `url` in `params`.
fn url_of(params: &str) -> &str {
    let rest = after(params, "url")
        .and_then(|r| r.strip_prefix('"'))
        .unwrap_or("");
    &rest[..rest.find('"').unwrap_or(rest.len())]
}

fn answer(out: &mut String, id: &str, result: &str) {
    out.push_str(&format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{result}}}\n"
    ));
}

/// `escouade.call`, with room for `cap` bytes of answer: how many it took, and its answer.
fn host_call(request: &str, cap: usize) -> (i64, String) {
    let mut buffer = vec![0u8; cap];
    // SAFETY: both buffers live through the call, which keeps within their lengths.
    let n = unsafe { call(request.as_ptr(), request.len(), buffer.as_mut_ptr(), cap) };
    let text = String::from_utf8_lossy(&buffer[..n.max(0) as usize]).into_owned();
    (n, text)
}

fn handle(me: &mut Reactor, line: &str, lines: usize, out: &mut String) {
    me.first.get_or_insert_with(|| line.to_string());
    if matches!(method_of(line), Some("proc/out" | "proc/exit")) {
        me.heard.push(line.to_string());
        return;
    }
    // What the host tells it (a piece of a stream, a timer): shown to the tests as it came.
    if id_of(line).is_none() {
        if matches!(
            method_of(line),
            Some("net/chunk" | "net/end" | "ws/message" | "ws/closed" | "timer/fired")
        ) {
            out.push_str(&format!(
                "{{\"jsonrpc\":\"2.0\",\"method\":\"probe/seen\",\"params\":{line}}}\n"
            ));
        }
        return;
    }
    let Some(id) = id_of(line) else { return };
    let Some(method) = method_of(line) else {
        // The answer of the host to a fetch of ours: it ends the call that waits, the line as it came.
        let own: u64 = id.parse().unwrap_or(0);
        if let Some(i) = me.fetches.iter().position(|(f, _)| *f == own) {
            let (_, waiting) = me.fetches.remove(i);
            answer(out, &waiting, line);
        }
        return;
    };
    match method {
        "initialize" => answer(out, id, "{\"protocolVersion\":1,\"capabilities\":{}}"),
        "shutdown" => answer(out, id, "null"),
        "probe/turns" => {
            let boot = me
                .first
                .as_deref()
                .is_some_and(|l| l.contains("escouade/boot"));
            let result = format!(
                "{{\"turns\":{},\"lines\":{lines},\"boot\":{boot}}}",
                me.turns
            );
            answer(out, id, &result);
        }
        "probe/fetch" => {
            me.next += 1;
            let own = 1_000_000 + me.next;
            me.fetches.push((own, id.to_string()));
            let url = url_of(params_of(line));
            out.push_str(&format!(
                "{{\"jsonrpc\":\"2.0\",\"id\":{own},\"method\":\"net/fetch\",\"params\":{{\"url\":\"{url}\"}}}}\n"
            ));
        }
        // Anything to the host, `{"method":…,"params":…}` as the tests wrote it: its answer ends the call.
        "probe/host" => {
            me.next += 1;
            let own = 1_000_000 + me.next;
            me.fetches.push((own, id.to_string()));
            let asked = params_of(line).strip_prefix('{').unwrap_or("}");
            out.push_str(&format!(
                "{{\"jsonrpc\":\"2.0\",\"id\":{own},{asked}\n"
            ));
        }
        "probe/call" => {
            let request = format!(
                "{{\"method\":\"probe/echo\",\"params\":{}}}",
                params_of(line)
            );
            let (n, text) = host_call(&request, 4096);
            answer(out, id, &format!("{{\"took\":{n},\"answer\":{text}}}"));
        }
        // A method of the host, by `escouade.call`: `{"method":…,"params":…}` as it came, its answer as it comes.
        "probe/sync" => {
            let (n, mut text) = host_call(params_of(line), 16 << 10);
            if n < 0 {
                text = host_call(params_of(line), (-n) as usize).1;
            }
            answer(out, id, &text);
        }
        // The same call twice, with no room for the first answer: the host answers the second without serving it again.
        "probe/retry" => {
            let (n, _) = host_call(params_of(line), 4);
            let (_, text) = host_call(params_of(line), (-n) as usize);
            answer(out, id, &text);
        }
        // What the programs wrote and how they ended, as it came.
        "probe/heard" => {
            let heard = me.heard.join(",");
            me.heard.clear();
            answer(out, id, &format!("[{heard}]"));
        }
        // An answer that does not fit says how much it needs.
        "probe/small" => {
            let (n, _) = host_call(
                "{\"method\":\"probe/echo\",\"params\":\"a text longer than its room\"}",
                4,
            );
            answer(out, id, &format!("{{\"took\":{n}}}"));
        }
        // A pointer out of its memory: a trap.
        "probe/wild" => {
            let mut buffer = [0u8; 8];
            // SAFETY: not safe: the host must refuse it.
            unsafe {
                call(usize::MAX as *const u8, 8, buffer.as_mut_ptr(), buffer.len());
            }
            answer(out, id, "null");
        }
        "probe/spin" => {
            let mut n = 0u64;
            loop {
                n = std::hint::black_box(n + 1);
            }
        }
        "probe/grow" => {
            let mut all = Vec::new();
            loop {
                all.push(vec![1u8; 1 << 20]);
                std::hint::black_box(&all);
            }
        }
        _ => out.push_str(&format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"error\":{{\"code\":-32601,\"message\":\"no such method\"}}}}\n"
        )),
    }
}

/// The host's turn: what came, its answers, and the end of the turn.
#[export_name = "escouade-turn"]
pub extern "C" fn turn() {
    let mut guard = state();
    let me = guard.as_mut().expect("made by state()");
    me.turns += 1;
    let mut buffer = [0u8; 16 * 1024];
    let mut all = std::mem::take(&mut me.partial);
    let mut input = std::io::stdin().lock();
    // An empty read is "nothing more for now", never an end.
    while let Ok(n) = input.read(&mut buffer) {
        if n == 0 {
            break;
        }
        all.extend_from_slice(&buffer[..n]);
    }
    let end = all.iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
    me.partial = all.split_off(end);
    let text = String::from_utf8_lossy(&all);
    let lines = text.lines().count();
    let mut out = String::new();
    for line in text.lines() {
        handle(me, line, lines, &mut out);
    }
    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(out.as_bytes());
    let _ = stdout.flush();
}
