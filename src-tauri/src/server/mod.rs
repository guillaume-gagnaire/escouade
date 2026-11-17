//! `escouade-server` (ESC-514, `docs/REMOTE.md`, « Le mode serveur »): the core without a window, for a machine that has
//! no desktop (a VPS, a container). It holds the data folder (`headless::InstanceLock`), loads the core
//! (`Headless::start_with`, its secrets in a file: `secrets`), serves the remote API on 127.0.0.1 (`Headless::serve_remote`),
//! writes its log (`logs`: the standard output and a file that turns), and stops cleanly at SIGTERM (the agents'
//! turns cut, the state written, as when the app quits: `Core::shutdown`).
//!
//! Its commands:
//! - `escouade-server [run]`: the server;
//! - `escouade-server pair`: a pairing code for a new device, once and briefly (`remoteapi::pairing`);
//! - `escouade-server devices [list]`, `devices revoke <id>`: the paired devices;
//! - `--data-dir <folder>` (else `~/.escouade`), `--port <n>` (else [`DEFAULT_PORT`]), `--listen <address>` (another
//!   interface than 127.0.0.1: for Docker, behind a tunnel or a proxy; it warns), `--debug`, `--version`, `--help`.
//!
//! Built without the desktop (`cargo build --bin escouade-server --no-default-features`: no WebKit, no GTK, docs/CRATES.md
//! § 14); it builds with it too (the e2e under Windows).

pub mod logs;
pub mod secrets;

use crate::headless::Headless;
use crate::host::HeadlessHost;
use crate::paths::DataDir;
use crate::remoteapi::devices::Devices;
use crate::remoteapi::pairing;
use crate::remoteapi::server::{Limits, Listen, RemoteApi};
use anyhow::{anyhow, bail, Context, Result};
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;

/// The port of the remote API when `--port` gives none.
pub const DEFAULT_PORT: u16 = 46480;

/// What the command line asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Run,
    Pair,
    Devices,
    Revoke(String),
    Version,
    Help,
}

/// The command line, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub command: Command,
    pub data_dir: Option<PathBuf>,
    pub port: u16,
    /// Another address than 127.0.0.1, asked for in so many words.
    pub listen: Option<IpAddr>,
    pub debug: bool,
}

/// The command line `args` (without the program's name). `--help` anywhere is the help, whatever else is there:
/// never a command run in its place.
pub fn parse(args: &[String]) -> std::result::Result<Args, String> {
    let mut out = Args {
        command: Command::Run,
        data_dir: None,
        port: DEFAULT_PORT,
        listen: None,
        debug: false,
    };
    if args
        .iter()
        .any(|a| a == "-h" || a == "--help" || a == "help")
    {
        out.command = Command::Help;
        return Ok(out);
    }
    if args
        .iter()
        .any(|a| a == "-V" || a == "--version" || a == "version")
    {
        out.command = Command::Version;
        return Ok(out);
    }
    let mut words = Vec::new();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v.to_string())),
            _ => (arg.as_str(), None),
        };
        let mut value = |name: &str| -> std::result::Result<String, String> {
            inline
                .clone()
                .or_else(|| it.next().cloned())
                .filter(|v| !v.is_empty())
                .ok_or_else(|| tr!("{name} attend une valeur", "{name} needs a value"))
        };
        match flag {
            "--data-dir" => out.data_dir = Some(PathBuf::from(value("--data-dir")?)),
            "--port" => {
                let v = value("--port")?;
                out.port = v.parse().map_err(|_| {
                    tr!(
                        "--port : « {v} » n’est pas un port",
                        "--port: “{v}” is not a port"
                    )
                })?;
            }
            "--listen" => {
                let v = value("--listen")?;
                out.listen = Some(v.parse().map_err(|_| {
                    tr!(
                        "--listen : « {v} » n’est pas une adresse IP",
                        "--listen: “{v}” is not an IP address"
                    )
                })?);
            }
            "--debug" => out.debug = true,
            f if f.starts_with('-') => {
                return Err(tr!("option inconnue : {f}", "unknown option: {f}"))
            }
            word => words.push(word.to_string()),
        }
    }
    out.command = match words.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        [] | ["run"] => Command::Run,
        ["pair"] => Command::Pair,
        ["devices"] | ["devices", "list"] => Command::Devices,
        ["devices", "revoke", id] => Command::Revoke(id.to_string()),
        ["devices", "revoke"] => {
            return Err(tr!(
                "devices revoke attend l’id d’un appareil (escouade-server devices)",
                "devices revoke needs a device's id (escouade-server devices)"
            ))
        }
        _ => {
            return Err(tr!(
                "commande inconnue : {} (escouade-server --help)",
                "unknown command: {} (escouade-server --help)",
                words.join(" ")
            ))
        }
    };
    Ok(out)
}

/// The help.
pub fn help() -> String {
    tr!(
        "escouade-server {version} : le cœur d’Escouade sans fenêtre, piloté à distance.\n\n\
         Usage :\n  \
         escouade-server [run]              sert l’API distante sur 127.0.0.1 (port {port})\n  \
         escouade-server pair               un code d’appairage pour un nouvel appareil (10 min, une fois)\n  \
         escouade-server devices            les appareils appairés\n  \
         escouade-server devices revoke ID  révoque un appareil\n\n\
         Options :\n  \
         --data-dir DOSSIER  le dossier des données (sinon ~/.escouade)\n  \
         --port N            le port de l’API distante (sinon {port})\n  \
         --listen ADRESSE    écoute sur une autre adresse que 127.0.0.1 (Docker, un tunnel, un proxy) : à ne jamais exposer sur Internet\n  \
         --debug             le journal en détail\n  \
         --version, --help",
        "escouade-server {version}: Escouade's core without a window, driven from afar.\n\n\
         Usage:\n  \
         escouade-server [run]              serves the remote API on 127.0.0.1 (port {port})\n  \
         escouade-server pair               a pairing code for a new device (10 min, once)\n  \
         escouade-server devices            the paired devices\n  \
         escouade-server devices revoke ID  revokes a device\n\n\
         Options:\n  \
         --data-dir FOLDER  the data folder (else ~/.escouade)\n  \
         --port N           the port of the remote API (else {port})\n  \
         --listen ADDRESS   listens on another address than 127.0.0.1 (Docker, a tunnel, a proxy): never to expose to the Internet\n  \
         --debug            a detailed log\n  \
         --version, --help",
        version = env!("CARGO_PKG_VERSION"),
        port = DEFAULT_PORT
    )
}

fn data_of(args: &Args) -> Result<DataDir> {
    let root = args
        .data_dir
        .clone()
        .unwrap_or_else(crate::paths::default_data_dir);
    let data = DataDir::new(root);
    data.ensure()
        .with_context(|| format!("{}", data.root().display()))?;
    Ok(data)
}

/// The languages of the settings of the data folder `args` names (else the default one), the system's when it has none:
/// what the commands print is in them, as the core will write.
fn speak_as_the_settings(args: &[String]) {
    let named = args
        .iter()
        .position(|a| a == "--data-dir")
        .and_then(|i| args.get(i + 1).cloned())
        .or_else(|| {
            args.iter()
                .find_map(|a| a.strip_prefix("--data-dir=").map(String::from))
        })
        .map(PathBuf::from);
    let root = named.unwrap_or_else(crate::paths::default_data_dir);
    let settings: serde_json::Value = std::fs::read(DataDir::new(root).settings_file())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let text = |key: &str| settings[key].as_str().unwrap_or("").to_string();
    crate::i18n::configure(&text("language"), &text("claudeLanguage"));
}

/// `escouade-server` with `args` (without the program's name): its exit code.
pub fn main(args: Vec<String>) -> i32 {
    speak_as_the_settings(&args);
    let args = match parse(&args) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("escouade-server: {e}");
            return 2;
        }
    };
    let done = match &args.command {
        Command::Help => {
            println!("{}", help());
            Ok(())
        }
        Command::Version => {
            println!("escouade-server {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::Pair => pair(&args),
        Command::Devices => devices(&args),
        Command::Revoke(id) => revoke(&args, id),
        Command::Run => run(&args),
    };
    match done {
        Ok(()) => 0,
        Err(e) => {
            log::error!("escouade-server: {e:#}");
            eprintln!("escouade-server: {e:#}");
            1
        }
    }
}

/// `pair`: a new code, printed for the user alone (never in the log).
fn pair(args: &Args) -> Result<()> {
    let data = data_of(args)?;
    let now = crate::model::now_ms();
    let code = pairing::create(&data, now)?;
    let until = chrono::DateTime::from_timestamp_millis(code.expires_at)
        .map(|t| t.with_timezone(&chrono::Local).format("%H:%M").to_string())
        .unwrap_or_default();
    println!(
        "{}",
        tr!(
            "Code d’appairage : {}\n\nValable une fois, {} minutes (jusqu’à {until}), {} essais faux au plus.\nDonne-le à Escouade sur le nouvel appareil (« Se connecter à un serveur »). Un nouveau « pair » remplace ce code.",
            "Pairing code: {}\n\nGood once, for {} minutes (until {until}), {} wrong tries at most.\nGive it to Escouade on the new device (“Connect to a server”). A new “pair” replaces this code.",
            code.text,
            pairing::LIFETIME_MS / 60_000,
            pairing::MAX_TRIES
        )
    );
    Ok(())
}

/// The devices of `data`, their tokens in the server's file.
fn devices_of(data: &DataDir) -> Result<Devices> {
    let store = Arc::new(secrets::FileStore::open(data)?);
    Ok(Devices::load(data, store))
}

/// `devices`: the list.
fn devices(args: &Args) -> Result<()> {
    let data = data_of(args)?;
    let list = devices_of(&data)?.list();
    if list.is_empty() {
        println!(
            "{}",
            tr!(
                "Aucun appareil appairé (escouade-server pair).",
                "No paired device (escouade-server pair)."
            )
        );
        return Ok(());
    }
    let date = |ms: i64| {
        chrono::DateTime::from_timestamp_millis(ms)
            .map(|t| {
                t.with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M")
                    .to_string()
            })
            .unwrap_or_default()
    };
    for d in list {
        let seen = d.last_seen.map(date).unwrap_or_else(|| "—".into());
        println!(
            "{}  {}  {}",
            d.id,
            d.name,
            tr!(
                "appairé le {}, vu le {seen}",
                "paired on {}, seen on {seen}",
                date(d.created_at)
            )
        );
    }
    Ok(())
}

/// `devices revoke <id>`: the device can no longer connect; a server that runs closes its connections within seconds.
fn revoke(args: &Args, id: &str) -> Result<()> {
    let data = data_of(args)?;
    if !devices_of(&data)?.revoke(id)? {
        bail!(tr!(
            "aucun appareil {id} (escouade-server devices)",
            "no device {id} (escouade-server devices)"
        ));
    }
    println!("{}", tr!("Appareil {id} révoqué.", "Device {id} revoked."));
    Ok(())
}

/// The folder of the resources beside the program (the seed of the plugins, `seed/`), when it has one.
fn resources() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(|d| d.to_path_buf())
        .filter(|d| d.join("seed").is_dir())
}

/// The stop the system asks for: SIGTERM (systemd, `docker stop`) or SIGINT (Ctrl+C); Ctrl+C under Windows.
async fn stop_asked() -> &'static str {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        match (
            signal(SignalKind::terminate()),
            signal(SignalKind::interrupt()),
        ) {
            (Ok(mut term), Ok(mut int)) => tokio::select! {
                _ = term.recv() => "SIGTERM",
                _ = int.recv() => "SIGINT",
            },
            _ => {
                let _ = tokio::signal::ctrl_c().await;
                "SIGINT"
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
        "Ctrl+C"
    }
}

/// The server, until the system asks it to stop.
fn run(args: &Args) -> Result<()> {
    let data = data_of(args)?;
    logs::install(data.root(), args.debug);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("escouade-server")
        .build()?;
    // What the core spawns runs on this runtime.
    tauri::async_runtime::set(runtime.handle().clone());
    let listen = args.listen;
    let port = args.port;
    let done = runtime.block_on(serve(
        data,
        listen,
        port,
        Limits::default(),
        |_| {},
        stop_asked(),
    ));
    runtime.shutdown_timeout(std::time::Duration::from_secs(5));
    done
}

/// Where the API listens: 127.0.0.1:`port`, or `listen`:`port`, asked for in so many words, said loudly.
fn listen_on(listen: Option<IpAddr>, port: u16) -> Result<Listen> {
    match listen {
        None => Ok(Listen::Port(port)),
        Some(ip) if ip.is_loopback() => {
            let socket = std::net::TcpListener::bind(SocketAddr::new(ip, port))
                .with_context(|| format!("{ip}:{port}"))?;
            Ok(Listen::Bound(socket))
        }
        Some(ip) => {
            let warning = tr!(
                "ATTENTION : l’API distante écoute sur {ip}:{port}, pas seulement sur 127.0.0.1 (--listen). Qui atteint ce port peut tenter de s’appairer : garde-le derrière un tunnel, un réseau privé ou un proxy, jamais ouvert sur Internet.",
                "WARNING: the remote API listens on {ip}:{port}, not on 127.0.0.1 alone (--listen). Whoever reaches this port may try to pair: keep it behind a tunnel, a private network or a proxy, never open to the Internet."
            );
            eprintln!("{warning}");
            log::warn!("{warning}");
            let socket = std::net::TcpListener::bind(SocketAddr::new(ip, port))
                .with_context(|| format!("{ip}:{port}"))?;
            Ok(Listen::Bound(socket))
        }
    }
}

/// The server on `data`: the core started, the API served (`ready` is told it), until `stop` comes; then the API
/// closed and the core stopped as when the app quits, its state written. On a runtime of Tokio.
pub async fn serve(
    data: DataDir,
    listen: Option<IpAddr>,
    port: u16,
    limits: Limits,
    ready: impl FnOnce(&Arc<RemoteApi>),
    stop: impl Future<Output = &'static str>,
) -> Result<()> {
    // The data folder first: nothing of it is touched while another process holds it.
    let lock = crate::headless::InstanceLock::acquire(data.root()).map_err(|e| anyhow!("{e:#}"))?;
    let store = Arc::new(secrets::FileStore::open(&data)?);
    let host = Arc::new(HeadlessHost::new(resources()).with_secrets(store.clone()));
    let root = data.root().to_path_buf();
    // The process groups of agents a killed server left behind, stopped before any agent starts (as the app).
    crate::agentgroups::start(root.join("agent-groups.json"));
    let headless = Headless::start_locked(lock, data, host);
    let core = headless.core.clone();
    let api = match headless
        .serve_remote_on(listen_on(listen, port)?, limits)
        .await
    {
        Ok(api) => api,
        Err(e) => {
            stop_core(headless).await;
            return Err(e);
        }
    };
    let addr = api.addr();
    let shown = match listen {
        Some(ip) if !ip.is_loopback() => addr.to_string(),
        _ => SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), addr.port()).to_string(),
    };
    log::info!(
        "escouade-server {} ready: remote API on ws://{shown}{}, data in {}, secrets in {}",
        env!("CARGO_PKG_VERSION"),
        crate::remoteapi::proto::PATH,
        root.display(),
        store.path().display()
    );
    // What the app does once its core is up, but its window (ESC-128, ESC-124, ESC-133).
    let migrating = core.clone();
    tauri::async_runtime::spawn(async move { migrating.migrate_accounts().await });
    let warm = core.clone();
    tauri::async_runtime::spawn_blocking(move || warm.warm_claude());
    core.read_all_account_models();
    crate::selfupdate::watch(core.clone());
    ready(&api);
    let why = stop.await;
    log::info!("escouade-server: {why}: stopping (the agents' turns are cut, the state written)");
    stop_core(headless).await;
    log::info!("escouade-server: stopped");
    Ok(())
}

/// The core stopped off the runtime's threads (its plugins are waited for): the API closed, the agents, terminals and
/// plugins stopped, the state written.
async fn stop_core(headless: Headless) {
    let stopped = tokio::task::spawn_blocking(move || headless.stop()).await;
    if let Err(e) = stopped {
        log::error!("escouade-server: the stop did not finish: {e}");
    }
}
