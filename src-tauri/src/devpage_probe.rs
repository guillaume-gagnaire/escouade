//! The probe of a launch's page in a real webview of this system (ESC-311): `cargo run --example
//! devpage_probe`. The unit tests run on Tauri's mock runtime and the end-to-end test on Windows
//! alone; this one runs the page as the app does, on WebView2, WKWebView or WebKitGTK, to say on
//! each system what the page could reach of Escouade, where its storage went, whether its
//! devtools opened, and the widths it saw, in the agent's view and in the « Pages » window.
//!
//! A local server serves a page that tries Escouade's commands (the app's and a plugin's), keeps
//! something in its storage and tells all of it back, then tells its width at each change. The
//! answer is a line of JSON; the exit code is 0 when the page was kept apart and every step held.
//! Its window fits a 1024 × 768 screen (the CI's macOS runner): a bigger one is narrowed by macOS
//! after the page is placed, and nothing here places it again, as the window's view « Page » does.

use crate::devpage::{self, Bounds, Open, PageMode, Pages};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use tauri::{Manager, Url, WebviewUrl, WebviewWindowBuilder};

const PAGE: &str = r#"<!doctype html><meta charset="utf-8"><title>sonde</title><h1>Sonde ESC-311</h1><script>
addEventListener('resize', () => fetch('/width?w=' + innerWidth));
fetch('/width?w=' + innerWidth);
(async () => {
  const r = { tauri: typeof window.__TAURI__ };
  const tryCall = async (cmd, args) => {
    try {
      const call = window.__TAURI_INTERNALS__.invoke(cmd, args);
      const answer = await Promise.race([call, new Promise((_, no) => setTimeout(() => no(new Error('no answer')), 3000))]);
      return 'answered: ' + JSON.stringify(answer);
    } catch (e) { return 'refused: ' + String((e && e.message) || e); }
  };
  r.app = await tryCall('open_url', { url: location.origin + '/pwned' });
  r.plugin = await tryCall('plugin:opener|open_url', { url: location.origin + '/pwned' });
  r.list = await tryCall('devpage_list', {});
  localStorage.setItem('mark', 'esc-311');
  r.storage = localStorage.getItem('mark');
  await fetch('/report', { method: 'POST', body: JSON.stringify(r) });
})();
</script>"#;

/// What the page tells the server.
enum Told {
    Report(String),
    Width(u32),
    Pwned,
}

/// Serves the page on a free port of the loopback; what it tells comes on `tx`.
fn serve(tx: mpsc::Sender<Told>) -> std::io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let tx = tx.clone();
            std::thread::spawn(move || {
                let mut reader = BufReader::new(&stream);
                let mut first = String::new();
                if reader.read_line(&mut first).is_err() {
                    return;
                }
                let mut length = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line.trim().is_empty() {
                        break;
                    }
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        length = v.trim().parse().unwrap_or(0);
                    }
                }
                let mut body = vec![0u8; length];
                let _ = reader.read_exact(&mut body);
                let path = first.split_whitespace().nth(1).unwrap_or("/").to_string();
                if path == "/report" {
                    let _ = tx.send(Told::Report(String::from_utf8_lossy(&body).into_owned()));
                } else if let Some(w) = path.strip_prefix("/width?w=") {
                    if let Ok(w) = w.parse() {
                        let _ = tx.send(Told::Width(w));
                    }
                } else if path == "/pwned" {
                    let _ = tx.send(Told::Pwned);
                }
                let html = if path == "/" { PAGE } else { "ok" };
                let _ = write!(
                    &stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}",
                    html.len()
                );
            });
        }
    });
    Ok(port)
}

/// What the probe saw, as its line of JSON says it.
#[derive(Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Seen {
    os: &'static str,
    report: Option<serde_json::Value>,
    pwned: bool,
    storage_dir: Option<bool>,
    /// None where the webview cannot say (WebView2 always answers false).
    devtools_open: Option<bool>,
    width_in_agent: Option<u32>,
    /// The page was still at a phone's width a few seconds later (nothing laid it out again).
    width_in_agent_held: bool,
    pages_window: bool,
    width_in_window: Option<u32>,
    /// Every width the page told, in order.
    widths: Vec<u32>,
}

impl Seen {
    fn kept_apart(&self) -> bool {
        let refused = |k: &str| {
            self.report
                .as_ref()
                .and_then(|r| r.get(k))
                .and_then(|v| v.as_str())
                .is_some_and(|v| v.starts_with("refused: "))
        };
        refused("app")
            && refused("plugin")
            && refused("list")
            && !self.pwned
            && self.report.as_ref().and_then(|r| r.get("storage")) == Some(&"esc-311".into())
            && self.storage_dir != Some(false)
            && self.devtools_open != Some(false)
            && self.width_in_agent == Some(390)
            && self.width_in_agent_held
            && self.pages_window
            && self.width_in_window == Some(768)
    }
}

/// Waits for what the page tells, until `want` takes something of it or `limit` passes.
fn wait<T>(
    rx: &mpsc::Receiver<Told>,
    seen: &mut Seen,
    limit: Duration,
    want: impl Fn(&Told) -> Option<T>,
) -> Option<T> {
    let end = std::time::Instant::now() + limit;
    while let Some(left) = end.checked_duration_since(std::time::Instant::now()) {
        match rx.recv_timeout(left) {
            Ok(Told::Pwned) => seen.pwned = true,
            Ok(told) => {
                if let Told::Width(w) = told {
                    seen.widths.push(w);
                }
                if let Some(t) = want(&told) {
                    return Some(t);
                }
            }
            Err(_) => return None,
        }
    }
    None
}

pub fn cli(args: Vec<String>, context: tauri::Context) -> i32 {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("devpage_probe: opens a launch's page in a real webview of this system and prints what it saw (ESC-311).");
        return 0;
    }
    let devtools_last = args.iter().any(|a| a == "--devtools-last");
    let (tx, rx) = mpsc::channel();
    let port = match serve(tx) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("no local server: {e}");
            return 2;
        }
    };
    let data = std::env::temp_dir().join(format!("escouade-devpage-probe-{}", std::process::id()));
    let code = Arc::new(std::sync::atomic::AtomicI32::new(1));
    let out = code.clone();
    let pages = Arc::new(Pages::default());
    let lanes = crate::windowthread::Lanes::new(Arc::new(crate::windowthread::Running::default()));
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(pages.clone())
        .invoke_handler(crate::windowthread::off_the_window(
            lanes,
            tauri::generate_handler![crate::commands::open_url, crate::commands::devpage_list],
        ))
        .setup(move |app| {
            WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External(Url::parse("about:blank")?),
            )
            .title("devpage_probe")
            .inner_size(900.0, 660.0)
            .build()?;
            let own: devpage::Own = Arc::new(|u: &Url| {
                crate::navigation::destination(u, None) == crate::navigation::Destination::App
            });
            let tell: devpage::Tell = Arc::new(|e| eprintln!("page: {} {}", e.phase, e.url));
            let url = format!("http://127.0.0.1:{port}/");
            devpage::open(
                app.handle(),
                &pages,
                Open {
                    agent_id: "probe",
                    project_id: "probe",
                    title: "probe · web",
                    url: &url,
                    detached: false,
                    bounds: Some(Bounds {
                        x: 0.0,
                        y: 0.0,
                        width: 900.0,
                        height: 600.0,
                    }),
                    data_root: &data,
                    visible: true,
                },
                own,
                tell,
            )
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            let (handle, pages, data) = (app.handle().clone(), pages.clone(), data.clone());
            std::thread::spawn(move || {
                let mut seen = Seen {
                    os: std::env::consts::OS,
                    ..Default::default()
                };
                let limit = Duration::from_secs(60);
                seen.report = wait(&rx, &mut seen, limit, |t| match t {
                    Told::Report(r) => serde_json::from_str(r).ok(),
                    _ => None,
                });
                seen.storage_dir = (!cfg!(target_os = "macos"))
                    .then(|| devpage::data_dir(&data, "probe").is_dir());
                // Its devtools, never the window's: first, as a user would, or last (`--devtools-last`).
                let devtools = |seen: &mut Seen| {
                    let _ = devpage::devtools(&handle, "probe");
                    // Long enough for WebKit's inspector to settle (macOS moves it out on its own).
                    std::thread::sleep(Duration::from_secs(5));
                    seen.devtools_open = (!cfg!(windows)).then(|| {
                        handle
                            .get_webview(&devpage::page_label("probe"))
                            .is_some_and(|w| w.is_devtools_open())
                    });
                };
                if !devtools_last {
                    devtools(&mut seen);
                }
                // A phone's width in the agent's view, centred: the window places it so.
                let _ = devpage::place(
                    &handle,
                    &pages,
                    "probe",
                    Some(Bounds {
                        x: 255.0,
                        y: 0.0,
                        width: 390.0,
                        height: 600.0,
                    }),
                );
                seen.width_in_agent = wait(&rx, &mut seen, Duration::from_secs(15), |t| match t {
                    Told::Width(390) => Some(390),
                    _ => None,
                });
                // Still there a few seconds later: nothing (the inspector) laid it out again.
                let before = seen.widths.len();
                let _ = wait(&rx, &mut seen, Duration::from_secs(3), |_| None::<()>);
                seen.width_in_agent_held = seen.width_in_agent.is_some()
                    && seen.widths[before..].iter().all(|w| *w == 390);
                // To the « Pages » window, a tablet's width there.
                seen.pages_window = devpage::detach(&handle, &pages, "probe", true).is_ok()
                    && handle.get_window(&devpage::window_label("probe")).is_some();
                let _ = devpage::set_mode(&handle, &pages, "probe", PageMode::Tablet);
                seen.width_in_window = wait(&rx, &mut seen, Duration::from_secs(15), |t| match t {
                    Told::Width(768) => Some(768),
                    _ => None,
                });
                if devtools_last {
                    devtools(&mut seen);
                }
                let ok = seen.kept_apart();
                println!("{}", serde_json::to_string(&seen).unwrap_or_default());
                out.store(if ok { 0 } else { 1 }, std::sync::atomic::Ordering::SeqCst);
                let _ = std::fs::remove_dir_all(&data);
                handle.exit(0);
            });
            Ok(())
        })
        .build(context);
    match app {
        Ok(app) => {
            app.run_return(|_, _| {});
        }
        Err(e) => {
            eprintln!("the app did not start: {e}");
            return 2;
        }
    }
    code.load(std::sync::atomic::Ordering::SeqCst)
}
