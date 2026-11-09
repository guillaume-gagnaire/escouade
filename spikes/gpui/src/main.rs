//! Prototype jetable de l'étude ESC-365 (docs/GPUI.md) : une fenêtre GPUI, sans webview, avec une colonne
//! de Kanban qu'on réordonne au glisser-déposer et des terminaux sur de vrais PTY (portable-pty, comme l'app),
//! émulés par `alacritty_terminal` (comme Zed). Il ne sert qu'à mesurer ; il n'est jamais livré.
//!
//! Lancement :
//!   gpui-spike                       la fenêtre, un terminal PowerShell
//!   gpui-spike --flood <n>           écrit n lignes colorées sur stdout (le gros flux, lancé DANS le terminal)
//! Variables :
//!   SPIKE_TERMINALS=<n>              n terminaux ouverts (grille), 1 par défaut
//!   SPIKE_BENCH=flood                une fois le shell prêt, lance le gros flux dans le 1er terminal et mesure
//!   SPIKE_FLOOD_LINES=<n>            lignes du flux (100 000 par défaut)
//!   SPIKE_OUT=<fichier>              où écrire les mesures (JSON, une par ligne)
//!   SPIKE_QUIT_AFTER=1               quitte après la mesure

use std::fs::OpenOptions;
use std::io::{BufWriter, Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use alacritty_terminal::event::EventListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::Config;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::vte::ansi::{Color, NamedColor, Processor};
use alacritty_terminal::Term;
use gpui::{
    div, prelude::*, px, rgb, size, App, Bounds, Context, FocusHandle, HighlightStyle, Hsla, KeyDownEvent, Rgba,
    SharedString, StyledText, Window, WindowBounds, WindowOptions,
};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};

const COLS: usize = 120;
const LINES: usize = 32;
const SENTINEL: &[u8] = b"FLOOD-DONE";

fn epoch_ms() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64() * 1000.0
}

fn report(line: String) {
    eprintln!("{line}");
    if let Ok(path) = std::env::var("SPIKE_OUT") {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(f, "{line}");
        }
    }
}

/// Le gros flux : des lignes de ~100 caractères, avec des couleurs SGR, comme un build ou un agent bavard.
fn flood(n: usize) {
    let out = std::io::stdout();
    let mut w = BufWriter::with_capacity(1 << 16, out.lock());
    for i in 0..n {
        let _ = writeln!(
            w,
            "\x1b[32m{i:>7}\x1b[0m compiling crate-{:<4} \x1b[33mwarning\x1b[0m: unused variable `x{}` in src/lib.rs:{}:{} lorem ipsum",
            i % 977,
            i % 13,
            i % 400,
            i % 80
        );
    }
}

struct Size {
    cols: usize,
    lines: usize,
}

impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.lines
    }
    fn screen_lines(&self) -> usize {
        self.lines
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

/// Ce que l'émulateur renvoie au shell (la réponse à `ESC[6n`, que ConPTY attend avant de rien afficher).
#[derive(Clone)]
struct Listener(Arc<Mutex<Box<dyn Write + Send>>>);
impl EventListener for Listener {
    fn send_event(&self, event: alacritty_terminal::event::Event) {
        if let alacritty_terminal::event::Event::PtyWrite(text) = event {
            let mut w = self.0.lock().unwrap();
            let _ = w.write_all(text.as_bytes());
            let _ = w.flush();
        }
    }
}

/// Un terminal : son émulateur (lu au rendu), ce que le fil du PTY a vu du flux.
struct Pane {
    term: Arc<FairMutex<Term<Listener>>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    dirty: Arc<AtomicBool>,
    bytes: Arc<AtomicU64>,
    /// Quand le fil du PTY a parsé la sentinelle (fin du flux), en ms depuis l'époque.
    done_at: Arc<Mutex<Option<f64>>>,
    /// Les premiers octets reçus (le shell est prêt).
    ready: Arc<AtomicBool>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
    _master: Box<dyn portable_pty::MasterPty + Send>,
}

impl Pane {
    fn spawn() -> anyhow_lite::Result<Pane> {
        let pty = native_pty_system();
        let pair = pty
            .openpty(PtySize { rows: LINES as u16, cols: COLS as u16, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| e.to_string())?;
        let mut cmd = CommandBuilder::new("powershell.exe");
        cmd.arg("-NoLogo");
        cmd.arg("-NoProfile");
        let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(pair.master.take_writer().map_err(|e| e.to_string())?));
        let mut config = Config::default();
        config.scrolling_history = 10_000;
        let term = Arc::new(FairMutex::new(Term::new(config, &Size { cols: COLS, lines: LINES }, Listener(writer.clone()))));
        let dirty = Arc::new(AtomicBool::new(true));
        let bytes = Arc::new(AtomicU64::new(0));
        let done_at = Arc::new(Mutex::new(None));
        let ready = Arc::new(AtomicBool::new(false));
        {
            let (term, dirty, bytes, done_at, ready) =
                (term.clone(), dirty.clone(), bytes.clone(), done_at.clone(), ready.clone());
            std::thread::spawn(move || {
                let mut parser: Processor = Processor::new();
                let mut buf = vec![0u8; 1 << 16];
                let mut tail: Vec<u8> = Vec::new();
                loop {
                    let n = match reader.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => n,
                    };
                    {
                        let mut t = term.lock();
                        parser.advance(&mut *t, &buf[..n]);
                    }
                    bytes.fetch_add(n as u64, Ordering::Relaxed);
                    ready.store(true, Ordering::Relaxed);
                    dirty.store(true, Ordering::Release);
                    // La sentinelle, à cheval sur deux lectures au besoin.
                    tail.extend_from_slice(&buf[..n]);
                    if tail.windows(SENTINEL.len()).any(|w| w == SENTINEL) {
                        let mut d = done_at.lock().unwrap();
                        if d.is_none() {
                            *d = Some(epoch_ms());
                        }
                    }
                    let keep = tail.len().saturating_sub(SENTINEL.len());
                    tail.drain(..keep);
                }
            });
        }
        Ok(Pane {
            term,
            writer,
            dirty,
            bytes,
            done_at,
            ready,
            _child: child,
            _master: pair.master,
        })
    }

    fn write(&self, s: &str) {
        let mut w = self.writer.lock().unwrap();
        let _ = w.write_all(s.as_bytes());
        let _ = w.flush();
    }
}

/// Un petit module d'erreur, pour ne pas tirer anyhow.
mod anyhow_lite {
    pub type Result<T> = std::result::Result<T, String>;
}

fn named(c: NamedColor) -> u32 {
    match c {
        NamedColor::Black => 0x1e1e2e,
        NamedColor::Red => 0xf38ba8,
        NamedColor::Green => 0xa6e3a1,
        NamedColor::Yellow => 0xf9e2af,
        NamedColor::Blue => 0x89b4fa,
        NamedColor::Magenta => 0xf5c2e7,
        NamedColor::Cyan => 0x94e2d5,
        NamedColor::White => 0xbac2de,
        NamedColor::BrightBlack => 0x585b70,
        NamedColor::BrightRed => 0xf38ba8,
        NamedColor::BrightGreen => 0xa6e3a1,
        NamedColor::BrightYellow => 0xf9e2af,
        NamedColor::BrightBlue => 0x89b4fa,
        NamedColor::BrightMagenta => 0xf5c2e7,
        NamedColor::BrightCyan => 0x94e2d5,
        NamedColor::BrightWhite => 0xffffff,
        _ => 0xcdd6f4,
    }
}

fn color(c: Color) -> Hsla {
    let hex = match c {
        Color::Named(n) => named(n),
        Color::Spec(rgb) => ((rgb.r as u32) << 16) | ((rgb.g as u32) << 8) | rgb.b as u32,
        Color::Indexed(i) if i < 16 => named(match i {
            0 => NamedColor::Black,
            1 => NamedColor::Red,
            2 => NamedColor::Green,
            3 => NamedColor::Yellow,
            4 => NamedColor::Blue,
            5 => NamedColor::Magenta,
            6 => NamedColor::Cyan,
            7 => NamedColor::White,
            8 => NamedColor::BrightBlack,
            9 => NamedColor::BrightRed,
            10 => NamedColor::BrightGreen,
            11 => NamedColor::BrightYellow,
            12 => NamedColor::BrightBlue,
            13 => NamedColor::BrightMagenta,
            14 => NamedColor::BrightCyan,
            _ => NamedColor::BrightWhite,
        }),
        Color::Indexed(_) => 0xcdd6f4,
    };
    let c: Rgba = rgb(hex);
    c.into()
}

/// Une carte qu'on traîne : son rang et son titre.
#[derive(Clone)]
struct DraggedCard {
    ix: usize,
    title: SharedString,
}

impl Render for DraggedCard {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(260.))
            .px_3()
            .py_2()
            .rounded_md()
            .bg(rgb(0x313244))
            .border_1()
            .border_color(rgb(0x89b4fa))
            .text_color(rgb(0xcdd6f4))
            .text_sm()
            .child(self.title.clone())
    }
}

struct Measures {
    started: Instant,
    first_frame: bool,
    renders: u64,
    /// Le flux en cours : quand il a été lancé, les rendus et le plus long écart entre deux rendus depuis.
    flood_sent: Option<f64>,
    flood_renders: u64,
    last_render: Option<Instant>,
    max_gap_ms: f64,
    max_lag_ms: f64,
    reported: bool,
    /// La mesure du flux, à compléter par la frame qui suit.
    pending: Option<String>,
}

struct Spike {
    cards: Vec<SharedString>,
    panes: Vec<Pane>,
    focus: FocusHandle,
    m: Measures,
}

impl Spike {
    fn new(window: &mut Window, cx: &mut Context<Self>, started: Instant) -> Self {
        let n: usize = std::env::var("SPIKE_TERMINALS").ok().and_then(|s| s.parse().ok()).unwrap_or(1).max(1);
        let panes: Vec<Pane> = (0..n).map(|_| Pane::spawn().expect("pty")).collect();
        let cards = [
            "ESC-365 Étude GPUI",
            "ESC-363 Réglages à distance",
            "ESC-364 Règles git d'OpenCode",
            "ESC-352 Vues des plugins",
            "ESC-347 Harness en module",
            "ESC-346 Codex en run.wasm",
            "ESC-340 CI des plugins",
            "ESC-336 Compiler run.wasm",
            "ESC-334 Kit Rust",
            "ESC-331 Hôte WASM",
            "ESC-330 Tout en WASM",
            "ESC-317 MCP du store",
        ]
        .into_iter()
        .map(SharedString::from)
        .collect();
        let focus = cx.focus_handle();
        window.focus(&focus, cx);

        // Le rafraîchissement : un rendu quand un terminal a changé (au plus ~120 fois par seconde), et la boucle
        // de l'interface sondée toutes les 20 ms (son retard dit sa fluidité, comme le setInterval de l'e2e).
        cx.spawn(async move |this, cx| {
            let mut last = Instant::now();
            loop {
                cx.background_executor().timer(Duration::from_millis(8)).await;
                let now = Instant::now();
                let lag = (now - last).as_secs_f64() * 1000.0 - 8.0;
                last = now;
                let r = this.update(cx, |this, cx| {
                    if this.m.flood_sent.is_some() && !this.m.reported {
                        this.m.max_lag_ms = this.m.max_lag_ms.max(lag);
                    }
                    if this.panes.iter().any(|p| p.dirty.swap(false, Ordering::Acquire)) {
                        cx.notify();
                    }
                    this.bench_step(cx);
                });
                if r.is_err() {
                    break;
                }
            }
        })
        .detach();

        Spike {
            cards,
            panes,
            focus,
            m: Measures {
                started,
                first_frame: false,
                renders: 0,
                flood_sent: None,
                flood_renders: 0,
                last_render: None,
                max_gap_ms: 0.0,
                max_lag_ms: 0.0,
                reported: false,
                pending: None,
            },
        }
    }

    fn bench_step(&mut self, cx: &mut Context<Self>) {
        if std::env::var("SPIKE_BENCH").as_deref() != Ok("flood") {
            return;
        }
        if self.m.flood_sent.is_none() {
            // Les shells ont dessiné leur invite, et on attend un moment de calme.
            if !self.panes.iter().all(|p| p.ready.load(Ordering::Relaxed)) || self.m.started.elapsed() < Duration::from_secs(5) {
                return;
            }
            let exe = std::env::current_exe().unwrap();
            let lines = std::env::var("SPIKE_FLOOD_LINES").unwrap_or_else(|_| "100000".into());
            // La sentinelle est coupée en deux dans la commande : son écho ne la contient pas.
            for pane in &self.panes {
                pane.write(&format!("& '{}' --flood {lines}; Write-Output ('FLOOD'+'-DONE')\r", exe.display()));
            }
            self.m.flood_sent = Some(epoch_ms());
            self.m.flood_renders = 0;
            self.m.max_gap_ms = 0.0;
            self.m.max_lag_ms = 0.0;
            self.m.last_render = None;
            return;
        }
        if self.m.reported {
            return;
        }
        // Fini quand tous les terminaux ont vu leur sentinelle : la dernière fait foi.
        let dones: Vec<Option<f64>> = self.panes.iter().map(|p| *p.done_at.lock().unwrap()).collect();
        if dones.iter().all(Option::is_some) {
            let done = dones.iter().flatten().cloned().fold(0.0, f64::max);
            if self.m.reported {
                return;
            }
            self.m.reported = true;
            let sent = self.m.flood_sent.unwrap();
            let bytes: u64 = self.panes.iter().map(|p| p.bytes.load(Ordering::Relaxed)).sum();
            let renders = self.m.flood_renders;
            let gap = self.m.max_gap_ms;
            let lag = self.m.max_lag_ms;
            // « Peint » : la frame qui suit la fin du parse (render la programme).
            self.m.pending = Some(format!(
                "{{\"kind\":\"flood\",\"parsed_ms\":{:.0},\"bytes\":{bytes},\"renders\":{renders},\"max_render_gap_ms\":{gap:.0},\"max_loop_lag_ms\":{lag:.0}",
                done - sent
            ));
            cx.notify();
        }
    }

    fn on_key(&mut self, ev: &KeyDownEvent, cx: &mut Context<Self>) {
        let k = &ev.keystroke;
        let pane = &self.panes[0];
        let s: String = if k.modifiers.control && k.key.len() == 1 {
            let c = k.key.as_bytes()[0].to_ascii_lowercase();
            if c.is_ascii_lowercase() { ((c - b'a' + 1) as char).to_string() } else { return }
        } else {
            match k.key.as_str() {
                "enter" => "\r".into(),
                "backspace" => "\x7f".into(),
                "tab" => "\t".into(),
                "escape" => "\x1b".into(),
                "up" => "\x1b[A".into(),
                "down" => "\x1b[B".into(),
                "right" => "\x1b[C".into(),
                "left" => "\x1b[D".into(),
                "space" => " ".into(),
                _ => match &k.key_char {
                    Some(c) => c.clone(),
                    None => return,
                },
            }
        };
        pane.write(&s);
        cx.stop_propagation();
    }

    fn column(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .w(px(290.))
            .h_full()
            .p_3()
            .bg(rgb(0x181825))
            .border_r_1()
            .border_color(rgb(0x313244))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .text_sm()
                    .text_color(rgb(0xa6adc8))
                    .child("En cours")
                    .child(format!("{}", self.cards.len())),
            )
            .children(self.cards.iter().enumerate().map(|(ix, title)| {
                let dragged = DraggedCard { ix, title: title.clone() };
                div()
                    .id(("card", ix))
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(rgb(0x1e1e2e))
                    .border_1()
                    .border_color(rgb(0x313244))
                    .text_color(rgb(0xcdd6f4))
                    .text_sm()
                    .cursor_move()
                    .hover(|s| s.border_color(rgb(0x585b70)))
                    .drag_over::<DraggedCard>(|s, _, _, _| s.border_color(rgb(0x89b4fa)))
                    .child(title.clone())
                    .on_drag(dragged, |d: &DraggedCard, _, _, cx| cx.new(|_| d.clone()))
                    .on_drop(cx.listener(move |this, d: &DraggedCard, _, cx| {
                        let card = this.cards.remove(d.ix);
                        let to = ix.min(this.cards.len());
                        this.cards.insert(to, card);
                        cx.notify();
                    }))
            }))
    }

    fn terminal(pane: &Pane, rows: usize) -> impl IntoElement {
        let term = pane.term.lock();
        let grid = term.grid();
        let screen = grid.screen_lines();
        let cols = grid.columns();
        // Les lignes qui finissent au curseur (l'invite d'un shell neuf est en haut, la fin d'un flux en bas).
        let cursor = (grid.cursor.point.line.0.max(0) as usize).min(screen.saturating_sub(1));
        let last = (cursor + 1).max(rows.min(screen));
        let first = last.saturating_sub(rows);
        let mut lines = Vec::with_capacity(rows);
        for l in first..last {
            let row = &grid[Line(l as i32)];
            let mut text = String::with_capacity(cols);
            let mut runs: Vec<(std::ops::Range<usize>, HighlightStyle)> = Vec::new();
            let mut run_start = 0;
            let mut run_fg: Option<Color> = None;
            for c in 0..cols {
                let cell = &row[Column(c)];
                if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                    continue;
                }
                if run_fg != Some(cell.fg) {
                    if let Some(fg) = run_fg {
                        if text.len() > run_start {
                            runs.push((run_start..text.len(), HighlightStyle { color: Some(color(fg)), ..Default::default() }));
                        }
                    }
                    run_start = text.len();
                    run_fg = Some(cell.fg);
                }
                text.push(cell.c);
            }
            if let Some(fg) = run_fg {
                if text.len() > run_start {
                    runs.push((run_start..text.len(), HighlightStyle { color: Some(color(fg)), ..Default::default() }));
                }
            }
            let trimmed = text.trim_end().len();
            runs.retain_mut(|(r, _)| {
                r.end = r.end.min(trimmed);
                r.start < r.end
            });
            text.truncate(trimmed);
            lines.push(div().h(px(17.)).whitespace_nowrap().overflow_hidden().child(StyledText::new(text).with_highlights(runs)));
        }
        drop(term);
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .p_2()
            .bg(rgb(0x11111b))
            .border_1()
            .border_color(rgb(0x313244))
            .font_family("Cascadia Mono")
            .text_size(px(13.))
            .text_color(rgb(0xcdd6f4))
            .children(lines)
    }
}

impl Render for Spike {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = Instant::now();
        self.m.renders += 1;
        if self.m.flood_sent.is_some() && !self.m.reported {
            self.m.flood_renders += 1;
            if let Some(last) = self.m.last_render {
                self.m.max_gap_ms = self.m.max_gap_ms.max((now - last).as_secs_f64() * 1000.0);
            }
            self.m.last_render = Some(now);
        }
        if !self.m.first_frame {
            self.m.first_frame = true;
            let started = self.m.started;
            window.on_next_frame(move |_, _| {
                report(format!(
                    "{{\"kind\":\"startup\",\"first_frame_epoch_ms\":{:.0},\"since_main_ms\":{:.0}}}",
                    epoch_ms(),
                    started.elapsed().as_secs_f64() * 1000.0
                ));
            });
        }
        if let Some(m) = self.m.pending.take() {
            let sent = self.m.flood_sent.unwrap_or(0.0);
            window.on_next_frame(move |_, _| {
                report(format!("{m},\"painted_ms\":{:.0}}}", epoch_ms() - sent));
                if std::env::var("SPIKE_QUIT_AFTER").is_ok() {
                    std::process::exit(0);
                }
            });
        }
        let n = self.panes.len();
        let rows = if n == 1 { LINES } else if n <= 4 { 14 } else { 8 };
        let per_row = if n == 1 { 1 } else { 2 };
        let mut grid_rows = Vec::new();
        let mut i = 0;
        while i < n {
            let mut r = div().flex().flex_row().gap_2().flex_1().min_h_0();
            for j in i..(i + per_row).min(n) {
                r = r.child(Self::terminal(&self.panes[j], rows));
            }
            grid_rows.push(r);
            i += per_row;
        }
        div()
            .size_full()
            .flex()
            .flex_row()
            .bg(rgb(0x1e1e2e))
            .font_family("Segoe UI")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| this.on_key(ev, cx)))
            .child(self.column(cx))
            .child(div().flex().flex_col().flex_1().min_w_0().gap_2().p_2().children(grid_rows))
    }
}

fn main() {
    let started = Instant::now();
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--flood") {
        flood(args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100_000));
        return;
    }
    gpui_platform::application().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
        cx.open_window(
            WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() },
            |window, cx| cx.new(|cx| Spike::new(window, cx, started)),
        )
        .unwrap();
        cx.activate(true);
    });
}
