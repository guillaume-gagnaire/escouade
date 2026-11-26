//! The probe of an agent's page in a real webview of this system (ESC-667, docs/AGENT-PAGES.md
//! § 8.4): `cargo run --example agentpage_probe`. The unit tests check the shelter's rules; this
//! one opens the trap (`agentpage_trap.html`) as the app does, on WebView2, WKWebView or
//! WebKitGTK, and says what it reached.
//!
//! Two listeners of the loopback (TCP and UDP) see whatever leaves the page: a fetch, an image, a
//! WebSocket, a frame, a prefetch, a preconnection, WebRTC's STUN or TURN, a navigation. The page
//! says the rest in its title: Escouade's commands and a plugin's, `eval`, its storage, the
//! permissions. Four openings:
//! 1. the trap, sheltered as the app shelters it: nothing may leave, nothing may answer;
//! 2. the same page opened again: its storage must be gone;
//! 3. and 4. the CSP in its header alone, then in its `<meta>` alone: what the webview does with
//!    each (said, not judged: the app always sends both).
//!
//! The answer is a line of JSON; the exit code is 0 when the first two held.

use crate::agentpage::{self, Bounds, Harden, Open, Shelter};
use std::io::Read;
use std::net::{TcpListener, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
#[cfg(target_os = "linux")]
use tauri::Manager;
use tauri::{State, Url, WebviewUrl, WebviewWindowBuilder};

/// What left the page, by the listeners.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "by")]
enum Hit {
    /// A TCP connection, with its first line when it sent one (`GET /fetch HTTP/1.1`).
    Tcp { line: String },
    /// A UDP datagram (STUN), its size.
    Udp { bytes: usize },
}

/// Starts the listeners; what reaches them comes on `tx`. Their ports: TCP, UDP.
fn listen(tx: mpsc::Sender<Hit>) -> std::io::Result<(u16, u16)> {
    let tcp = TcpListener::bind("127.0.0.1:0")?;
    let udp = UdpSocket::bind("127.0.0.1:0")?;
    let ports = (tcp.local_addr()?.port(), udp.local_addr()?.port());
    let to = tx.clone();
    std::thread::spawn(move || {
        for stream in tcp.incoming().flatten() {
            let to = to.clone();
            std::thread::spawn(move || {
                let mut stream = stream;
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let mut buf = [0u8; 512];
                let n = stream.read(&mut buf).unwrap_or(0);
                let text = String::from_utf8_lossy(&buf[..n]);
                let line = text.lines().next().unwrap_or("").to_string();
                let _ = to.send(Hit::Tcp { line });
            });
        }
    });
    std::thread::spawn(move || {
        let mut buf = [0u8; 2048];
        while let Ok((n, _)) = udp.recv_from(&mut buf) {
            let _ = tx.send(Hit::Udp { bytes: n });
        }
    });
    Ok(ports)
}

/// One opening of a page, as the probe saw it.
#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Run {
    page: String,
    harden: &'static str,
    report: Option<serde_json::Value>,
    hits: Vec<Hit>,
    view: Option<agentpage::PageView>,
    touched: bool,
    /// The last step the trap told before its report (`escouade-step:`), when it never gave one.
    #[serde(skip_serializing_if = "Option::is_none")]
    stopped_at: Option<String>,
}

impl Run {
    fn said(&self, key: &str) -> String {
        self.report
            .as_ref()
            .and_then(|r| r.get(key))
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .unwrap_or_default()
    }

    /// What did not hold, empty when the page was kept apart.
    fn breaches(&self) -> Vec<String> {
        let mut no = Vec::new();
        let mut want = |ok: bool, what: &str| {
            if !ok {
                no.push(what.to_string());
            }
        };
        want(self.report.is_some(), "the trap never said what it got");
        want(
            self.said("own") == "reached: own file",
            "its own file was not served",
        );
        want(self.hits.is_empty(), "something reached the network");
        want(!self.touched, "a command of Escouade answered");
        for k in ["app", "plugin", "touch", "list"] {
            want(
                self.said(k).starts_with("refused"),
                &format!("{k} answered"),
            );
        }
        for k in [
            "fetch",
            "img",
            "eval",
            "evalIndirect",
            "evalComplex",
            "timer",
            "fn",
            "camera",
            "microphone",
            "clipboard",
            "notification",
            "geolocation",
        ] {
            want(
                self.said(k).starts_with("refused"),
                &format!("{k} was not refused"),
            );
        }
        let other = self.said("otherPage");
        want(
            !other.contains("secret") && !other.starts_with("reached: 200"),
            "another page was read",
        );
        want(
            !self.said("otherPath").starts_with("reached: 200"),
            "a path out of the page was served",
        );
        want(
            !self.said("choose").starts_with("reached: 200"),
            "a channel answered",
        );
        want(
            self.said("rtc") == "undefined/undefined",
            "RTCPeerConnection is in the page",
        );
        want(
            self.said("rtcCall").starts_with("refused"),
            "a peer connection was made",
        );
        let wants = self
            .view
            .as_ref()
            .and_then(|v| v.wants.clone())
            .unwrap_or_default();
        want(
            wants.contains("/navigation?secret=1"),
            "its navigation without a gesture was not kept for the window",
        );
        no
    }
}

/// What the probe saw, as its line of JSON says it.
#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Seen {
    os: &'static str,
    runs: Vec<Run>,
    /// What did not hold in the sheltered openings (1 and 2).
    breaches: Vec<String>,
    /// Whether the webview honours the CSP of a header of the scheme alone (opening 3), and of the
    /// `<meta>` alone (opening 4): what docs/AGENT-PAGES.md § 8.4 left to the probe.
    csp_header_alone: Option<bool>,
    csp_meta_alone: Option<bool>,
    /// The connections Escouade's dead end cut (`agentpage::dead_end`; none on macOS): what the
    /// page reached for that nothing else stopped.
    cut: u64,
}

/// Whether an opening with the CSP of one kind alone kept the network out.
fn network_held(run: &Run) -> Option<bool> {
    run.report.as_ref()?;
    Some(
        run.hits.is_empty()
            && run.said("fetch").starts_with("refused")
            && run.said("img").starts_with("refused")
            && run.said("eval").starts_with("refused"),
    )
}

/// A command of Escouade, in the probe: its call would say the page reached it.
#[tauri::command]
fn probe_touch(touched: State<Arc<AtomicBool>>) {
    touched.store(true, Ordering::SeqCst);
}

pub fn cli(args: Vec<String>, context: tauri::Context) -> i32 {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("agentpage_probe: opens the trap of an agent's page in a real webview of this system and prints what reached out (ESC-667).");
        return 0;
    }
    let (hit_tx, hits) = mpsc::channel();
    let (tcp, udp) = match listen(hit_tx) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("no listener: {e}");
            return 2;
        }
    };
    let data =
        std::env::temp_dir().join(format!("escouade-agentpage-probe-{}", std::process::id()));
    let shelter = Arc::new(Shelter::default());
    let prepared = (|| -> std::io::Result<()> {
        for (page, harden) in [
            ("p-1", Harden::default()),
            (
                "p-2",
                Harden {
                    header: true,
                    meta: false,
                },
            ),
            (
                "p-3",
                Harden {
                    header: false,
                    meta: true,
                },
            ),
        ] {
            let dir = data.join(page);
            agentpage::write_trap(&dir, tcp, udp)?;
            let _ = shelter.put_hardened(page, dir, harden);
        }
        agentpage::write_neighbour(&data.join("p-13"))?;
        let _ = shelter.put("p-13", data.join("p-13"));
        Ok(())
    })();
    if let Err(e) = prepared {
        eprintln!("the trap was not written: {e}");
        return 2;
    }
    let code = Arc::new(AtomicI32::new(1));
    let out = code.clone();
    let touched = Arc::new(AtomicBool::new(false));
    let lanes = crate::windowthread::Lanes::new(Arc::new(crate::windowthread::Running::default()));
    let builder = agentpage::register(tauri::Builder::default());
    let app = builder
        .plugin(tauri_plugin_opener::init())
        .manage(shelter.clone())
        .manage(touched.clone())
        .invoke_handler(crate::windowthread::off_the_window(
            lanes,
            tauri::generate_handler![
                crate::commands::open_url,
                crate::commands::agentpage_view,
                probe_touch
            ],
        ))
        .setup(move |app| {
            WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External(Url::parse("about:blank")?),
            )
            .title("agentpage_probe")
            .inner_size(900.0, 660.0)
            .build()?;
            let handle = app.handle().clone();
            let (title_tx, titles) = mpsc::channel::<(String, String)>();
            let tell: agentpage::Tell = Arc::new(move |e| {
                if e.phase == "title" {
                    eprintln!(
                        "page {}: title {}",
                        e.page,
                        e.title.chars().take(60).collect::<String>()
                    );
                    let _ = title_tx.send((e.page.clone(), e.title.clone()));
                } else {
                    eprintln!("page {}: {} {}", e.page, e.phase, e.url);
                }
            });
            let data = data.clone();
            std::thread::spawn(move || {
                let mut seen = Seen {
                    os: std::env::consts::OS,
                    ..Default::default()
                };
                let own: crate::devpage::Own = Arc::new(|u: &Url| {
                    crate::navigation::destination(u, None) == crate::navigation::Destination::App
                });
                for (page, harden) in [
                    ("p-1", "both"),
                    ("p-1", "both, opened again"),
                    ("p-2", "header alone"),
                    ("p-3", "meta alone"),
                ] {
                    while hits.try_recv().is_ok() {}
                    while titles.try_recv().is_ok() {}
                    touched.store(false, Ordering::SeqCst);
                    let mut run = Run {
                        page: page.to_string(),
                        harden,
                        ..Default::default()
                    };
                    let opened = agentpage::open(
                        &handle,
                        &shelter,
                        Open {
                            page,
                            bounds: Some(Bounds {
                                x: 0.0,
                                y: 0.0,
                                width: 900.0,
                                height: 600.0,
                            }),
                            visible: true,
                            data_root: &data,
                        },
                        own.clone(),
                        tell.clone(),
                    );
                    if let Err(e) = opened {
                        eprintln!("page {page} did not open: {e}");
                    }
                    // WebKitGTK has no API for the console: its messages go to the probe's output.
                    #[cfg(target_os = "linux")]
                    if let Some(w) = handle.get_webview(&agentpage::label(page)) {
                        let _ = w.with_webview(|p| {
                            use webkit2gtk::{SettingsExt, WebViewExt};
                            if let Some(s) = WebViewExt::settings(&p.inner()) {
                                s.set_enable_write_console_messages_to_stdout(true);
                            }
                        });
                    }
                    let end = Instant::now() + Duration::from_secs(60);
                    // The report's parts (`escouade-part:<i>/<n>:<text>`), put together.
                    let mut parts: Vec<Option<String>> = Vec::new();
                    while let Some(left) = end.checked_duration_since(Instant::now()) {
                        match titles.recv_timeout(left) {
                            Ok((p, t)) if p == page => {
                                if let Some(json) = t.strip_prefix("escouade-probe:") {
                                    run.report = serde_json::from_str(json)
                                        .map_err(|e| {
                                            eprintln!(
                                                "page {page}: its report is not JSON ({e}): {json}"
                                            )
                                        })
                                        .ok();
                                    run.stopped_at = None;
                                    break;
                                }
                                if let Some(step) = t.strip_prefix("escouade-step:") {
                                    run.stopped_at = Some(step.to_string());
                                }
                                if let Some((at, text)) = t
                                    .strip_prefix("escouade-part:")
                                    .and_then(|rest| rest.split_once(':'))
                                {
                                    if let Some((i, n)) = at.split_once('/').and_then(|(i, n)| {
                                        Some((i.parse::<usize>().ok()?, n.parse::<usize>().ok()?))
                                    }) {
                                        if parts.len() != n {
                                            parts = vec![None; n];
                                        }
                                        if i < n {
                                            parts[i] = Some(text.to_string());
                                        }
                                        if parts.iter().all(Option::is_some) {
                                            let whole: String =
                                                parts.iter().flatten().cloned().collect();
                                            run.report = serde_json::from_str(&whole).ok();
                                            if run.report.is_some() {
                                                run.stopped_at = None;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            Ok(_) => {}
                            Err(_) => break,
                        }
                    }
                    // Its navigation without a gesture, and what comes late (a prefetch, STUN).
                    std::thread::sleep(Duration::from_secs(4));
                    run.hits = hits.try_iter().collect();
                    run.view = shelter.view(page);
                    run.touched = touched.load(Ordering::SeqCst);
                    agentpage::close(&handle, &shelter, page);
                    // Long enough for a webview's process to let its private session go.
                    std::thread::sleep(Duration::from_secs(2));
                    seen.runs.push(run);
                }
                let mut breaches: Vec<String> = Vec::new();
                for run in &seen.runs[..2] {
                    breaches.extend(
                        run.breaches()
                            .into_iter()
                            .map(|b| format!("{} ({}): {b}", run.page, run.harden)),
                    );
                }
                if seen.runs[1].said("storageBefore") != "null" {
                    breaches.push("p-1 kept its storage from one opening to the next".into());
                }
                seen.breaches = breaches;
                seen.csp_header_alone = network_held(&seen.runs[2]);
                seen.csp_meta_alone = network_held(&seen.runs[3]);
                seen.cut = agentpage::cut();
                let ok = seen.breaches.is_empty();
                println!("{}", serde_json::to_string(&seen).unwrap_or_default());
                out.store(if ok { 0 } else { 1 }, Ordering::SeqCst);
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
    code.load(Ordering::SeqCst)
}
