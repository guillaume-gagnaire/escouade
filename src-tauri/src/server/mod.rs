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
//! - `escouade-server team pair <serveur> <CODE> [--name <nom>]` (OE-1): the machine enters a team as a machine of the
//!   team (the role of service), by the short code an administrator shows (`teampair`);
//! - `escouade-server health`: whether the server of the folder runs and still answers (the image's `HEALTHCHECK`);
//! - `escouade-server update [--to X.Y.Z [--downgrade]] [--beta] [--now] [--check]`: the server at a signed version (`update`);
//! - `escouade-server config [automatic on|off] [channel stable|beta]` (ESC-702): the automatic update and its channel,
//!   the server's settings (`autoupdate`);
//! - `escouade-server phone [status]`, `phone on [PORT]`, `phone off`, `phone add NAME [--endpoint HOST]`,
//!   `phone remove ID` (ESC-567): the phone's tunnel, its file changed, read again by a server that runs; `add` prints
//!   the phone's WireGuard config and its QR code for the user alone (the official WireGuard app, until MO-4's pairing);
//! - `escouade-server via <host> <port>` (RP-1): `ssh`'s `ProxyCommand` through the gate (`escouade_gate::via`), which
//!   the core puts in `GIT_SSH_COMMAND` and `core.sshCommand`; nothing else read, nothing logged;
//! - `--gate` (`ESCOUADE_SERVER_GATE`, RP-1): the gate of the private network on 127.0.0.1, the agents' one proxy
//!   (`privatenet`); off by default;
//! - `--connector-listen <adresse:port>` (`ESCOUADE_SERVER_CONNECTOR_LISTEN`, RP-4): the connectors' entry, off by
//!   default, which speaks their protocol alone (never the remote API), and the gate with it (`privateentry`);
//!   `--connector-cert`, `--connector-key` (a certificate of its own, else behind a TLS proxy), `--connector-url`
//!   (its address as the connectors reach it);
//! - `escouade-server connector pair <nom>`, `connectors [list]`, `connectors remove <nom>` (RP-4): a connector's
//!   pairing code (8 letters, 10 minutes, once), the connectors paired and their state, one removed;
//! - `--data-dir <folder>` (else `~/.escouade`), `--port <n>` (else [`DEFAULT_PORT`]), `--listen <address>` (another
//!   interface than 127.0.0.1: for Docker, behind a tunnel or a proxy; it warns), `--allow-host <name>` (a `Host`
//!   the API takes besides the loopback's: the name a TLS proxy serves it under), `--trust-proxy <address>` (a proxy
//!   whose `X-Forwarded-For` says where a connection comes from: its clients' failures counted apart, SR1-04),
//!   `--browser` (BC-1: the page of the browser's client on the API's port, off by default), `--debug`, `--version`,
//!   `--help`.
//! - Their variables, for a container (ESC-515): `ESCOUADE_SERVER_DATA`, `ESCOUADE_SERVER_PORT`, `ESCOUADE_SERVER_LISTEN`,
//!   `ESCOUADE_SERVER_ALLOW_HOSTS` (names separated by commas), `ESCOUADE_SERVER_TRUST_PROXIES` (addresses separated by
//!   commas), `ESCOUADE_SERVER_GATE` and `ESCOUADE_SERVER_BROWSER` (`1`); an option given wins over its variable. So
//!   `docker exec <container> escouade-server pair` finds the folder of the server it runs beside.
//!
//! Built without the desktop (`cargo build --bin escouade-server --no-default-features`: no WebKit, no GTK, docs/CRATES.md
//! § 14); it builds with it too (the e2e under Windows).

pub mod autoupdate;
pub mod logs;
pub mod secrets;
pub mod status;
pub mod teampair;
pub mod update;

use crate::headless::Headless;
use crate::host::HeadlessHost;
use crate::paths::DataDir;
use crate::remoteapi::devices::Devices;
use crate::remoteapi::page::Page;
use crate::remoteapi::pairing;
use crate::remoteapi::server::{Limits, Listen, RemoteApi};
use anyhow::{anyhow, bail, Context, Result};
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;

/// The port of the remote API when `--port` gives none.
pub const DEFAULT_PORT: u16 = 46480;

/// The variables that stand for the options (an option given wins).
pub const DATA_VAR: &str = "ESCOUADE_SERVER_DATA";
pub const PORT_VAR: &str = "ESCOUADE_SERVER_PORT";
pub const LISTEN_VAR: &str = "ESCOUADE_SERVER_LISTEN";
pub const HOSTS_VAR: &str = "ESCOUADE_SERVER_ALLOW_HOSTS";
pub const PROXIES_VAR: &str = "ESCOUADE_SERVER_TRUST_PROXIES";
/// `--gate` (RP-1): `1`, `true`, `on` or `yes`.
pub const GATE_VAR: &str = "ESCOUADE_SERVER_GATE";
/// `--browser` (BC-1): `1`, `true`, `on` or `yes`.
pub const BROWSER_VAR: &str = "ESCOUADE_SERVER_BROWSER";
/// `--connector-listen` and its company (RP-4).
pub const CONNECTOR_LISTEN_VAR: &str = "ESCOUADE_SERVER_CONNECTOR_LISTEN";
pub const CONNECTOR_CERT_VAR: &str = "ESCOUADE_SERVER_CONNECTOR_CERT";
pub const CONNECTOR_KEY_VAR: &str = "ESCOUADE_SERVER_CONNECTOR_KEY";
pub const CONNECTOR_URL_VAR: &str = "ESCOUADE_SERVER_CONNECTOR_URL";

/// What the command line asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Run,
    Pair,
    Devices,
    Revoke(String),
    Health,
    Update,
    /// `config …` (ESC-702): the automatic update (`automatic`) and the channel (`beta`), each when given.
    Config {
        automatic: Option<bool>,
        beta: Option<bool>,
    },
    /// `phone …` (ESC-567).
    Phone(PhoneCommand),
    /// `team pair <serveur> <CODE>` (OE-1): enters a team as a machine of the team.
    TeamPair {
        server: String,
        code: String,
    },
    /// `connector pair <nom>` (RP-4): a connector's pairing code.
    ConnectorPair(String),
    /// `connectors [list]` (RP-4).
    Connectors,
    /// `connectors remove <nom>` (RP-4).
    ConnectorRemove(String),
    Version,
    Help,
}

/// What `escouade-server phone` asks (ESC-567).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhoneCommand {
    Status,
    On(Option<u16>),
    Off,
    Add(String),
    Remove(String),
}

/// The command line, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub command: Command,
    pub data_dir: Option<PathBuf>,
    pub port: u16,
    /// Another address than 127.0.0.1, asked for in so many words.
    pub listen: Option<IpAddr>,
    /// The names the API takes in a `Host` besides the loopback's (`--allow-host`).
    pub hosts: Vec<String>,
    /// The addresses of the proxies whose `X-Forwarded-For` is read (`--trust-proxy`, SR1-04).
    pub proxies: Vec<IpAddr>,
    pub debug: bool,
    /// The gate of the private network (`--gate`, RP-1): the agents’ one proxy.
    pub gate: bool,
    /// The page of the browser's client on the API's port (`--browser`, BC-1): off by default.
    pub browser: bool,
    /// What `update` is asked.
    pub update: update::Asked,
    /// Where a phone reaches the server (`phone add --endpoint`, `pair --qr --endpoint`): its name or address on the
    /// Internet.
    pub endpoint: Option<String>,
    /// `pair --qr` (ESC-570): a phone's QR code, in characters, in place of a computer's code.
    pub qr: bool,
    /// The name of the team on this machine (`team pair --name`; else its server's address).
    pub name: Option<String>,
    /// The connectors' entry (RP-4): `--connector-listen`, its certificate, its address.
    pub connector: ConnectorArgs,
}

/// What the command line says of the connectors' entry (RP-4).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConnectorArgs {
    pub listen: Option<SocketAddr>,
    pub cert: Option<PathBuf>,
    pub key: Option<PathBuf>,
    pub url: Option<String>,
}

/// The command line `args` (without the program's name), with the process's variables. `--help` anywhere is the
/// help, whatever else is there: never a command run in its place.
pub fn parse(args: &[String]) -> std::result::Result<Args, String> {
    parse_with(args, |name| std::env::var(name).ok())
}

/// The same, with the variables `var` gives.
pub fn parse_with(
    args: &[String],
    var: impl Fn(&str) -> Option<String>,
) -> std::result::Result<Args, String> {
    let var = |name: &str| {
        var(name)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    let mut out = Args {
        command: Command::Run,
        data_dir: var(DATA_VAR).map(PathBuf::from),
        port: DEFAULT_PORT,
        listen: None,
        hosts: Vec::new(),
        proxies: Vec::new(),
        debug: false,
        gate: var(GATE_VAR).is_some_and(|v| on(&v)),
        browser: var(BROWSER_VAR).is_some_and(|v| on(&v)),
        update: update::Asked::default(),
        endpoint: None,
        qr: false,
        name: None,
        connector: ConnectorArgs::default(),
    };
    if let Some(v) = var(CONNECTOR_LISTEN_VAR) {
        out.connector.listen = Some(listen_of(&v)?);
    }
    out.connector.cert = var(CONNECTOR_CERT_VAR).map(PathBuf::from);
    out.connector.key = var(CONNECTOR_KEY_VAR).map(PathBuf::from);
    if let Some(v) = var(CONNECTOR_URL_VAR) {
        out.connector.url = Some(connector_url_of(&v)?);
    }
    if let Some(v) = var(PORT_VAR) {
        out.port = port_of(&v)?;
    }
    if let Some(v) = var(LISTEN_VAR) {
        out.listen = Some(ip_of(&v)?);
    }
    if let Some(v) = var(HOSTS_VAR) {
        for h in v.split(',').map(str::trim).filter(|h| !h.is_empty()) {
            out.hosts.push(host_of(h)?);
        }
    }
    if let Some(v) = var(PROXIES_VAR) {
        for p in v.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            out.proxies.push(proxy_of(p)?);
        }
    }
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
    let mut update_flags = false;
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
            "--port" => out.port = port_of(&value("--port")?)?,
            "--listen" => out.listen = Some(ip_of(&value("--listen")?)?),
            "--allow-host" => {
                let v = host_of(&value("--allow-host")?)?;
                out.hosts.push(v);
            }
            "--trust-proxy" => {
                let v = proxy_of(&value("--trust-proxy")?)?;
                out.proxies.push(v);
            }
            "--debug" => out.debug = true,
            "--gate" => out.gate = true,
            "--browser" => out.browser = true,
            "--connector-listen" => {
                out.connector.listen = Some(listen_of(&value("--connector-listen")?)?)
            }
            "--connector-cert" => {
                out.connector.cert = Some(PathBuf::from(value("--connector-cert")?))
            }
            "--connector-key" => out.connector.key = Some(PathBuf::from(value("--connector-key")?)),
            "--connector-url" => {
                out.connector.url = Some(connector_url_of(&value("--connector-url")?)?)
            }
            "--endpoint" => out.endpoint = Some(value("--endpoint")?),
            "--qr" => out.qr = true,
            "--name" => out.name = Some(value("--name")?),
            "--to" => {
                update_flags = true;
                out.update.to = Some(value("--to")?);
            }
            "--beta" => {
                update_flags = true;
                out.update.beta = true;
            }
            "--now" => {
                update_flags = true;
                out.update.now = true;
            }
            "--check" => {
                update_flags = true;
                out.update.check = true;
            }
            "--downgrade" => {
                update_flags = true;
                out.update.downgrade = true;
            }
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
        ["health"] => Command::Health,
        ["update"] => Command::Update,
        ["config"] => Command::Config {
            automatic: None,
            beta: None,
        },
        ["config", ..] => config_of(&words[1..])?,
        ["phone"] | ["phone", "status"] => Command::Phone(PhoneCommand::Status),
        ["phone", "on"] => Command::Phone(PhoneCommand::On(None)),
        ["phone", "on", port] => Command::Phone(PhoneCommand::On(Some(port_of(port)?))),
        ["phone", "off"] => Command::Phone(PhoneCommand::Off),
        ["phone", "add", name] => Command::Phone(PhoneCommand::Add(name.to_string())),
        ["phone", "remove", id] => Command::Phone(PhoneCommand::Remove(id.to_string())),
        ["team", "pair", server, code] => Command::TeamPair {
            server: server.to_string(),
            code: code.to_string(),
        },
        ["connector", "pair", ref name @ ..] if !name.is_empty() => {
            Command::ConnectorPair(name.join(" "))
        }
        ["connector", ..] => {
            return Err(tr!(
                "connector pair attend le nom du connecteur (escouade-server connector pair Acme)",
                "connector pair needs the connector's name (escouade-server connector pair Acme)"
            ))
        }
        ["connectors"] | ["connectors", "list"] => Command::Connectors,
        ["connectors", "remove", ref name @ ..] if !name.is_empty() => {
            Command::ConnectorRemove(name.join(" "))
        }
        ["connectors", "remove"] => {
            return Err(tr!(
                "connectors remove attend le nom ou l’id d’un connecteur (escouade-server connectors)",
                "connectors remove needs a connector's name or id (escouade-server connectors)"
            ))
        }
        ["team", ..] => {
            return Err(tr!(
                "team pair attend l’adresse du serveur d’équipe et le code que montre l’administrateur (escouade-server team pair team.acme.fr K7Q4M-2XZ8P)",
                "team pair needs the team server's address and the code the administrator shows (escouade-server team pair team.acme.fr K7Q4M-2XZ8P)"
            ))
        }
        ["phone", "add"] | ["phone", "remove"] => {
            return Err(tr!(
                "phone add attend un nom, phone remove l’id d’un téléphone (escouade-server phone)",
                "phone add needs a name, phone remove a phone's id (escouade-server phone)"
            ))
        }
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
    if out.qr && out.command != Command::Pair {
        return Err(tr!("--qr va avec « pair »", "--qr goes with “pair”"));
    }
    if out.name.is_some() && !matches!(out.command, Command::TeamPair { .. }) {
        return Err(tr!(
            "--name va avec « team pair »",
            "--name goes with “team pair”"
        ));
    }
    if update_flags && out.command != Command::Update {
        return Err(tr!(
            "--to, --beta, --now, --check et --downgrade vont avec « update »",
            "--to, --beta, --now, --check and --downgrade go with “update”"
        ));
    }
    // An older version is taken only when it is named (SR1-07): never from latest.json nor beta.json.
    if out.update.downgrade && out.update.to.is_none() {
        return Err(tr!(
            "--downgrade va avec --to X.Y.Z : la version plus ancienne se nomme",
            "--downgrade goes with --to X.Y.Z: the older version is named"
        ));
    }
    if let Some(to) = &out.update.to {
        if !update::is_version(to.trim_start_matches('v')) {
            return Err(tr!(
                "--to : « {to} » n’est pas une version (X.Y.Z)",
                "--to: “{to}” is not a version (X.Y.Z)"
            ));
        }
    }
    Ok(out)
}

/// A variable that turns something on: `1`, `true`, `on` or `yes` (any case).
fn on(v: &str) -> bool {
    matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "on" | "yes")
}

/// The address of the connectors' entry (`--connector-listen`): `adresse:port`, an IP address and its port.
fn listen_of(v: &str) -> std::result::Result<SocketAddr, String> {
    v.trim().parse().map_err(|_| {
        tr!(
            "--connector-listen : « {v} » n’est pas une adresse:port (0.0.0.0:8443, [::]:8443, 127.0.0.1:8443)",
            "--connector-listen: “{v}” is not an address:port (0.0.0.0:8443, [::]:8443, 127.0.0.1:8443)"
        )
    })
}

/// The entry's address as the connectors reach it (`--connector-url`): `wss://…` (or `ws://` for a test).
fn connector_url_of(v: &str) -> std::result::Result<String, String> {
    let v = v.trim();
    let ok = escouade_gate::entry::proto::pairing::pairing_url(v).is_some();
    if !ok {
        return Err(tr!(
            "--connector-url : « {v} » n’est pas une adresse wss:// (wss://escouade.exemple.fr:8443/connector)",
            "--connector-url: “{v}” is not a wss:// address (wss://escouade.exemple.fr:8443/connector)"
        ));
    }
    Ok(v.to_string())
}

/// The words after `config`: `automatic on|off`, `channel stable|beta`, one or both.
fn config_of(words: &[String]) -> std::result::Result<Command, String> {
    let wrong = || {
        tr!(
            "config attend « automatic on|off » ou « channel stable|beta » (escouade-server config)",
            "config needs “automatic on|off” or “channel stable|beta” (escouade-server config)"
        )
    };
    let (mut automatic, mut beta) = (None, None);
    let mut it = words.iter().map(String::as_str);
    while let Some(word) = it.next() {
        match (word, it.next()) {
            ("automatic" | "auto", Some("on" | "true" | "1")) => automatic = Some(true),
            ("automatic" | "auto", Some("off" | "false" | "0")) => automatic = Some(false),
            ("channel", Some("stable")) => beta = Some(false),
            ("channel", Some("beta")) => beta = Some(true),
            _ => return Err(wrong()),
        }
    }
    Ok(Command::Config { automatic, beta })
}

fn port_of(v: &str) -> std::result::Result<u16, String> {
    v.parse().map_err(|_| {
        tr!(
            "--port : « {v} » n’est pas un port",
            "--port: “{v}” is not a port"
        )
    })
}

fn ip_of(v: &str) -> std::result::Result<IpAddr, String> {
    v.parse().map_err(|_| {
        tr!(
            "--listen : « {v} » n’est pas une adresse IP",
            "--listen: “{v}” is not an IP address"
        )
    })
}

/// The address of a proxy (`--trust-proxy`).
fn proxy_of(v: &str) -> std::result::Result<IpAddr, String> {
    v.trim().parse().map_err(|_| {
        tr!(
            "--trust-proxy : « {v} » n’est pas une adresse IP",
            "--trust-proxy: “{v}” is not an IP address"
        )
    })
}

/// A host name (`--allow-host`), in lower case: letters, digits, dots and dashes; never one of the loopback's, which
/// the API takes already.
fn host_of(v: &str) -> std::result::Result<String, String> {
    let v = v.trim().to_ascii_lowercase();
    if v.is_empty()
        || !v
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return Err(tr!(
            "--allow-host : « {v} » n’est pas un nom d’hôte",
            "--allow-host: “{v}” is not a host name"
        ));
    }
    Ok(v)
}

/// The help.
pub fn help() -> String {
    tr!(
        "escouade-server {version} : le cœur d’Escouade sans fenêtre, piloté à distance.\n\n\
         Usage :\n  \
         escouade-server [run]              sert l’API distante sur 127.0.0.1 (port {port})\n  \
         escouade-server pair               un code d’appairage pour un nouvel appareil (10 min, une fois)\n  \
         escouade-server pair --qr          le QR code d’un téléphone (10 min, une fois ; --endpoint HÔTE)\n  \
         escouade-server devices            les appareils appairés\n  \
         escouade-server devices revoke ID  révoque un appareil\n  \
         escouade-server health             le serveur tourne-t-il et répond-il ? (code 0 si oui)\n  \
         escouade-server update             met le serveur à la dernière version signée (--to X.Y.Z [--downgrade], --beta, --now, --check)\n  \
         escouade-server config             la mise à jour automatique et son canal (automatic on|off, channel stable|beta)\n  \
         escouade-server phone              le tunnel WireGuard du téléphone (on [PORT], off, add NOM [--endpoint HÔTE], remove ID)\n  \
         escouade-server team pair SERVEUR CODE  entre dans une équipe comme machine de l’équipe, par le code que montre un administrateur (--name NOM : son nom ici)\n  \
         escouade-server via HÔTE PORT      le ProxyCommand de ssh par la porte (ProxyCommand=\"escouade-server via %h %p\")\n  \
         escouade-server connector pair NOM le code d’appairage d’un connecteur du réseau privé (8 lettres, 10 min, une fois)\n  \
         escouade-server connectors         les connecteurs appairés, leur état, et ceux retirés\n  \
         escouade-server connectors remove NOM  retire un connecteur : sa clé ne vaut plus, son canal est fermé\n\n\
         Options :\n  \
         --data-dir DOSSIER  le dossier des données (sinon ~/.escouade)\n  \
         --port N            le port de l’API distante (sinon {port})\n  \
         --listen ADRESSE    écoute sur une autre adresse que 127.0.0.1 (Docker, un tunnel, un proxy) : à ne jamais exposer sur Internet\n  \
         --allow-host NOM    accepte ce nom dans l’en-tête Host (celui d’un proxy TLS devant le serveur)\n  \
         --trust-proxy IP    lit X-Forwarded-For de ce proxy seul : chaque client y compte ses échecs à part\n  \
         --gate              la porte du réseau privé : le seul proxy des agents, sur 127.0.0.1\n  \
         --browser           sert l’interface dans un navigateur sur le port de l’API (éteint sinon) ; derrière un proxy, en https:// seulement\n  \
         --connector-listen ADRESSE:PORT  l’entrée des connecteurs (éteinte sinon), et la porte avec elle : le seul protocole des connecteurs, jamais l’API distante\n  \
         --connector-cert FICHIER, --connector-key FICHIER  son certificat TLS (sinon, derrière un proxy TLS)\n  \
         --connector-url wss://…  son adresse telle que les connecteurs la joignent\n  \
         --debug             le journal en détail\n  \
         --version, --help",
        "escouade-server {version}: Escouade's core without a window, driven from afar.\n\n\
         Usage:\n  \
         escouade-server [run]              serves the remote API on 127.0.0.1 (port {port})\n  \
         escouade-server pair               a pairing code for a new device (10 min, once)\n  \
         escouade-server pair --qr          a phone's QR code (10 min, once; --endpoint HOST)\n  \
         escouade-server devices            the paired devices\n  \
         escouade-server devices revoke ID  revokes a device\n  \
         escouade-server health             does the server run and answer? (exit code 0 if so)\n  \
         escouade-server update             puts the server at the latest signed version (--to X.Y.Z [--downgrade], --beta, --now, --check)\n  \
         escouade-server config             the automatic update and its channel (automatic on|off, channel stable|beta)\n  \
         escouade-server phone              the phone's WireGuard tunnel (on [PORT], off, add NAME [--endpoint HOST], remove ID)\n  \
         escouade-server team pair SERVER CODE  enters a team as a machine of the team, by the code an administrator shows (--name NAME: its name here)\n  \
         escouade-server via HOST PORT      ssh's ProxyCommand through the gate (ProxyCommand=\"escouade-server via %h %p\")\n  \
         escouade-server connector pair NAME  a pairing code for a connector of the private network (8 letters, 10 min, once)\n  \
         escouade-server connectors         the paired connectors, their state, and those removed\n  \
         escouade-server connectors remove NAME  removes a connector: its key no longer holds, its channel is closed\n\n\
         Options:\n  \
         --data-dir FOLDER  the data folder (else ~/.escouade)\n  \
         --port N           the port of the remote API (else {port})\n  \
         --listen ADDRESS   listens on another address than 127.0.0.1 (Docker, a tunnel, a proxy): never to expose to the Internet\n  \
         --allow-host NAME  takes this name in the Host header (a TLS proxy's in front of the server)\n  \
         --trust-proxy IP   reads X-Forwarded-For from this proxy alone: each client behind it counts its failures apart\n  \
         --gate             the gate of the private network: the agents' one proxy, on 127.0.0.1\n  \
         --browser          serves the interface to a browser on the API's port (off otherwise); behind a proxy, in https:// alone\n  \
         --connector-listen ADDRESS:PORT  the connectors' entry (off otherwise), and the gate with it: the connectors' protocol alone, never the remote API\n  \
         --connector-cert FILE, --connector-key FILE  its TLS certificate (else, behind a TLS proxy)\n  \
         --connector-url wss://…  its address as the connectors reach it\n  \
         --debug            a detailed log\n  \
         --version, --help",
        version = env!("CARGO_PKG_VERSION"),
        port = DEFAULT_PORT
    )
}

/// The data folder `args` names (else the default one), made or put back in mode 0700 under Unix (`DataDir::ensure`,
/// SR1-13): a server started by hand, without systemd's `UMask=0077` nor the image's `/data`, keeps it to its user.
pub(crate) fn data_of(args: &Args) -> Result<DataDir> {
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
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var(DATA_VAR)
                .ok()
                .filter(|v| !v.trim().is_empty())
                .map(PathBuf::from)
        });
    let root = named.unwrap_or_else(crate::paths::default_data_dir);
    let settings: serde_json::Value = std::fs::read(DataDir::new(root).settings_file())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let text = |key: &str| settings[key].as_str().unwrap_or("").to_string();
    crate::i18n::configure(&text("language"), &text("claudeLanguage"));
}

/// `escouade-server` with `args` (without the program's name): its exit code. No page of the browser's client.
pub fn main(args: Vec<String>) -> i32 {
    main_with(args, Web::default())
}

/// The page of the browser's client a binary carries (`escouade-web`, BC-1): its files (`(name, gzip)`) and its mark.
#[derive(Debug, Clone, Copy, Default)]
pub struct Web {
    pub files: &'static [(&'static str, &'static [u8])],
    pub mark: &'static str,
}

/// `escouade-server` with `args`, and the page `web` it serves with `--browser`: its exit code.
pub fn main_with(args: Vec<String>, web: Web) -> i32 {
    // `ssh`’s ProxyCommand (RP-1): nothing else read, nothing logged; its refusal on its standard error.
    if args.first().map(String::as_str) == Some("via") {
        return escouade_gate::via::main(&args[1..]);
    }
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
        Command::Health => health(&args),
        Command::Update => data_of(&args).and_then(|data| update::run(&data, &args.update)),
        Command::Config { automatic, beta } => config(&args, *automatic, *beta),
        Command::Phone(what) => phone(&args, what),
        Command::TeamPair { server, code } => team_pair(&args, server, code),
        Command::ConnectorPair(name) => connector_pair(&args, name),
        Command::Connectors => connectors(&args),
        Command::ConnectorRemove(name) => connector_remove(&args, name),
        Command::Run => run(&args, web),
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

/// `team pair <serveur> <CODE>` (OE-1): the request left for the server of the folder, its answer waited for, the six
/// words printed for the user alone (never in the log), to compare with the administrator's screen.
fn team_pair(args: &Args, server: &str, code: &str) -> Result<()> {
    let data = data_of(args)?;
    let running = status::read(&data)
        .is_some_and(|s| status::healthy(&s, crate::model::now_ms(), status::alive).is_ok());
    if !running {
        bail!(tr!(
            "aucun serveur lancé sur {} : lance « escouade-server » d’abord, puis « team pair »",
            "no server running on {}: start “escouade-server” first, then “team pair”",
            data.root().display()
        ));
    }
    let id = hex::encode(crate::model::now_ms().to_be_bytes());
    let request = teampair::Request {
        server: server.trim().to_string(),
        code: code.trim().to_string(),
        name: args.name.clone().unwrap_or_default(),
        id: id.clone(),
    };
    teampair::ask(data.root(), &request)?;
    println!(
        "{}",
        tr!(
            "Code transmis au serveur ; il attend l’appareil de l’administrateur…",
            "Code given to the server; it waits for the administrator's device…"
        )
    );
    let answer = teampair::wait(data.root(), &id, teampair::WAIT)?;
    if let Some(e) = answer.error {
        bail!(e);
    }
    println!(
        "{}",
        tr!(
            "Demande d’entrée dans l’équipe « {} », comme machine de l’équipe.\n\nLes six mots de cette machine : {}\n\nL’administrateur les compare avec ceux de son écran, puis « Admettre ». La machine entre alors, sous le rôle de service : elle lit et écrit les projets qu’on lui ouvre et prend des tickets, sans rien signer d’administratif.",
            "Asked to enter the team “{}”, as a machine of the team.\n\nThis machine's six words: {}\n\nThe administrator compares them with those of their screen, then “Admit”. The machine then enters, under the role of service: it reads and writes the projects opened to it and takes tickets, signing nothing of the administrators'.",
            answer.team.unwrap_or_default(),
            answer.words.join(" ")
        )
    );
    Ok(())
}

/// The policies of this device on the private network (RP-4): what a command beside the server checks first.
fn check_private_network() -> Result<()> {
    let managed = crate::locks::Managed::read();
    crate::locks::Locks::compute(crate::locks::KEYS, None, &managed).check_private_network()
}

/// A date of the connectors, in local time.
fn connector_date(ms: u64) -> String {
    chrono::DateTime::from_timestamp_millis(ms as i64)
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_default()
}

/// `connector pair <nom>` (RP-4): a code for the connector to be named `name`, printed for the user alone (never in the
/// log), with the entry's address and the connector's command. Refused when a policy denies the private network.
fn connector_pair(args: &Args, name: &str) -> Result<()> {
    check_private_network()?;
    let data = data_of(args)?;
    let store = Arc::new(secrets::FileStore::open(&data)?);
    let registry = crate::privateentry::registry_of(&data, store);
    let code = registry
        .create_code(name, crate::model::now_ms() as u64)
        .map_err(anyhow::Error::msg)?;
    let until = chrono::DateTime::from_timestamp_millis(code.expires_at as i64)
        .map(|t| t.with_timezone(&chrono::Local).format("%H:%M").to_string())
        .unwrap_or_default();
    let url = args.connector.url.clone().or_else(|| registry.entry_url());
    let minutes = escouade_gate::entry::registry::CODE_LIFETIME_MS / 60_000;
    let name = name.trim();
    println!(
        "{}",
        tr!(
            "Code d’appairage du connecteur « {name} » : {}\n\nValable une fois, {minutes} minutes (jusqu’à {until}) ; un code faux ne l’annule pas. Un nouveau « connector pair {name} » remplace ce code.",
            "Pairing code of the connector “{name}”: {}\n\nGood once, for {minutes} minutes (until {until}); a wrong code does not void it. A new “connector pair {name}” replaces this code.",
            code.text
        )
    );
    match url {
        Some(url) => println!(
            "{}",
            tr!(
                "Sur la machine du connecteur : escouade-connector pair {url} {}",
                "On the connector's machine: escouade-connector pair {url} {}",
                code.text
            )
        ),
        None => println!(
            "{}",
            tr!(
                "Sur la machine du connecteur : escouade-connector pair wss://<ce serveur>:<port>/connector {}\n(aucune entrée des connecteurs ouverte : lance escouade-server avec --connector-listen, et --connector-url pour l’adresse qu’elle a vue d’ailleurs)",
                "On the connector's machine: escouade-connector pair wss://<this server>:<port>/connector {}\n(no connectors' entry open: start escouade-server with --connector-listen, and --connector-url for its address as seen from elsewhere)",
                code.text
            )
        ),
    }
    Ok(())
}

/// `connectors` (RP-4): the connectors paired, their state, and the traces of those removed.
fn connectors(args: &Args) -> Result<()> {
    let data = data_of(args)?;
    let store = Arc::new(secrets::FileStore::open(&data)?);
    let registry = crate::privateentry::registry_of(&data, store);
    let views = registry.views();
    if views.is_empty() {
        println!(
            "{}",
            tr!(
                "Aucun connecteur appairé (escouade-server connector pair NOM).",
                "No paired connector (escouade-server connector pair NAME)."
            )
        );
    }
    use escouade_gate::entry::{State, VersionState};
    for v in &views {
        let state = match (v.state, v.since) {
            (State::Connected, Some(s)) => tr!(
                "connecté depuis {}",
                "connected since {}",
                connector_date(s)
            ),
            (State::Connected, None) => tr!("connecté", "connected"),
            (State::Down, Some(s)) => tr!("coupé depuis {}", "cut since {}", connector_date(s)),
            (State::Down, None) => tr!("coupé", "cut"),
            (State::Never, _) => tr!("jamais connecté", "never connected"),
        };
        let version = match (&v.version, v.version_state) {
            (Some(ver), Some(VersionState::Outdated)) => {
                tr!("{ver}, à mettre à jour", "{ver}, to update")
            }
            (Some(ver), Some(VersionState::TooOld)) => {
                tr!("{ver}, trop ancien (refusé)", "{ver}, too old (refused)")
            }
            (Some(ver), Some(VersionState::TooNew)) => tr!(
                "{ver}, plus récent que le serveur (refusé)",
                "{ver}, newer than the server (refused)"
            ),
            (Some(ver), _) => ver.clone(),
            (None, _) => "—".into(),
        };
        println!(
            "{}  {}  {state}  escouade-connector {version}  {}",
            v.id,
            v.name,
            tr!("plafond : {} règles", "ceiling: {} rules", v.ceiling.len())
        );
        if let Some(advice) = &v.advice {
            println!("    {advice}");
        }
    }
    let removed = registry.removed();
    if !removed.is_empty() {
        println!("\n{}", tr!("Retirés :", "Removed:"));
        for r in removed.iter().rev() {
            println!(
                "{}  {}  {}",
                r.id,
                r.name,
                tr!(
                    "retiré le {} par {}",
                    "removed on {} by {}",
                    connector_date(r.at),
                    r.by
                )
            );
        }
    }
    Ok(())
}

/// `connectors remove <nom>` (RP-4): its key no longer holds; a server that runs closes its channel within seconds.
fn connector_remove(args: &Args, which: &str) -> Result<()> {
    let data = data_of(args)?;
    let store = Arc::new(secrets::FileStore::open(&data)?);
    let registry = crate::privateentry::registry_of(&data, store);
    let removed = registry
        .remove(
            which,
            escouade_gate::entry::registry::BY_COMMAND,
            crate::model::now_ms() as u64,
        )
        .map_err(anyhow::Error::msg)?;
    let Some(r) = removed else {
        bail!(tr!(
            "aucun connecteur « {which} » (escouade-server connectors)",
            "no connector “{which}” (escouade-server connectors)"
        ));
    };
    println!(
        "{}",
        tr!(
            "Connecteur « {} » retiré : sa clé ne vaut plus, et un serveur qui tourne ferme son canal dans les secondes qui viennent.",
            "Connector “{}” removed: its key no longer holds, and a server that runs closes its channel within seconds.",
            r.name
        )
    );
    Ok(())
}

/// `pair`: a new code, printed for the user alone (never in the log).
fn pair(args: &Args) -> Result<()> {
    let data = data_of(args)?;
    if args.qr {
        return pair_qr(args, &data);
    }
    let now = crate::model::now_ms();
    let code = pairing::create(&data, now)?;
    let until = chrono::DateTime::from_timestamp_millis(code.expires_at)
        .map(|t| t.with_timezone(&chrono::Local).format("%H:%M").to_string())
        .unwrap_or_default();
    println!(
        "{}",
        tr!(
            "Code d’appairage : {}\n\nValable une fois, {} minutes (jusqu’à {until}) ; un code faux ne l’annule pas.\nDonne-le à Escouade sur le nouvel appareil (« Se connecter à un serveur »). Un nouveau « pair » remplace ce code.",
            "Pairing code: {}\n\nGood once, for {} minutes (until {until}); a wrong code does not void it.\nGive it to Escouade on the new device (“Connect to a server”). A new “pair” replaces this code.",
            code.text,
            pairing::LIFETIME_MS / 60_000
        )
    );
    Ok(())
}

/// `pair --qr` (ESC-570): a phone's QR code, in characters, and its code beside it, for the user alone (never in the
/// log): the phone's WireGuard config, its private key included. The phone's tunnel must be on.
fn pair_qr(args: &Args, data: &DataDir) -> Result<()> {
    let store = Arc::new(secrets::FileStore::open(data)?);
    let phone = crate::phone::Phone::load(data, store);
    phone.mark_server();
    let qr = phone.pair_qr(data, None, false, args.endpoint.as_deref())?;
    let drawn = qrcode::QrCode::with_error_correction_level(qr.text.as_bytes(), qrcode::EcLevel::M)
        .map(|c| {
            c.render::<qrcode::render::unicode::Dense1x2>()
                .quiet_zone(true)
                .build()
        })
        .unwrap_or_default();
    let until = chrono::DateTime::from_timestamp_millis(qr.expires_at)
        .map(|t| t.with_timezone(&chrono::Local).format("%H:%M").to_string())
        .unwrap_or_default();
    println!(
        "{drawn}\n{}",
        tr!(
            "Dans l’app Escouade du téléphone : « Scanner le QR code ». Code : {}\nValable une fois, {} minutes (jusqu’à {until}). Le téléphone joindra {}{}.\nCe QR porte la clé du téléphone : ne le photographie pas, ne le partage pas.",
            "In Escouade’s app on the phone: “Scan the QR code”. Code: {}\nGood once, for {} minutes (until {until}). The phone will reach {}{}.\nThis QR holds the phone’s key: do not photograph it, do not share it.",
            qr.code,
            crate::remoteapi::pairing::LIFETIME_MS / 60_000,
            qr.endpoint,
            if qr.endpoint_unknown {
                tr!(" (à vérifier : --endpoint HÔTE)", " (to check: --endpoint HOST)")
            } else {
                String::new()
            }
        )
    );
    Ok(())
}

/// `phone …` (ESC-567): the phone's tunnel of the folder, its file changed (a server that runs reads it again within
/// seconds). `add` prints the phone's private key, in its WireGuard config and its QR code, for the user alone: never in
/// the log, kept nowhere.
fn phone(args: &Args, what: &PhoneCommand) -> Result<()> {
    let data = data_of(args)?;
    let store = Arc::new(secrets::FileStore::open(&data)?);
    let phone = crate::phone::Phone::load(&data, store);
    match what {
        PhoneCommand::Status => {
            let s = phone.summary();
            println!(
                "{}",
                if s.enabled {
                    tr!(
                        "Tunnel du téléphone : allumé, port UDP {}.",
                        "Phone tunnel: on, UDP port {}.",
                        s.port
                    )
                } else {
                    tr!(
                        "Tunnel du téléphone : éteint (port UDP {}).",
                        "Phone tunnel: off (UDP port {}).",
                        s.port
                    )
                }
            );
            if let Some(key) = &s.public_key {
                println!("{}", tr!("Clé publique : {key}", "Public key: {key}"));
            }
            for p in phone.peers() {
                println!("{}  {}  {}", p.id, p.name, p.address);
            }
        }
        PhoneCommand::On(port) => {
            phone.set_saved(Some(true), *port)?;
            println!(
                "{}",
                tr!(
                    "Tunnel du téléphone allumé sur le port UDP {} : ouvre-le dans le pare-feu du serveur.",
                    "Phone tunnel on, on UDP port {}: open it in the server's firewall.",
                    phone.summary().port
                )
            );
        }
        PhoneCommand::Off => {
            phone.set_saved(Some(false), None)?;
            println!(
                "{}",
                tr!("Tunnel du téléphone éteint.", "Phone tunnel off.")
            );
        }
        PhoneCommand::Add(name) => {
            let (config, peer) = phone.add_phone(name, args.endpoint.as_deref())?;
            let qr = qrcode::QrCode::new(config.as_bytes())
                .map(|c| {
                    c.render::<qrcode::render::unicode::Dense1x2>()
                        .quiet_zone(true)
                        .build()
                })
                .unwrap_or_default();
            println!(
                "{}\n\n{qr}\n\n{config}\n{}",
                tr!(
                    "Téléphone « {} » ajouté ({}). Dans l'app WireGuard : + › Scanner un QR code.",
                    "Phone “{}” added ({}). In the WireGuard app: + › Scan from QR code.",
                    peer.name,
                    peer.address
                ),
                tr!(
                    "Cette config porte la clé privée du téléphone : elle n'est gardée nulle part, ne la partage pas.",
                    "This config holds the phone's private key: it is kept nowhere, do not share it."
                )
            );
        }
        PhoneCommand::Remove(id) => {
            let Some(peer) = phone.take_peer(id)? else {
                bail!(tr!(
                    "aucun téléphone {id} (escouade-server phone)",
                    "no phone {id} (escouade-server phone)"
                ));
            };
            // Its device's token revoked with it (SR1-03): it no longer passes by any way.
            match peer.device {
                Some(device) => {
                    devices_of(&data)?.revoke(&device)?;
                    println!(
                        "{}",
                        tr!(
                            "Téléphone {id} retiré et son accès révoqué.",
                            "Phone {id} removed and its access revoked."
                        )
                    );
                }
                None => println!(
                    "{}",
                    tr!(
                        "Téléphone {id} retiré du tunnel (il n'était lié à aucun appareil).",
                        "Phone {id} removed from the tunnel (it was tied to no device)."
                    )
                ),
            }
        }
    }
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

/// `config` (ESC-702): the automatic update and its channel changed when asked, then said.
fn config(args: &Args, automatic: Option<bool>, beta: Option<bool>) -> Result<()> {
    let data = data_of(args)?;
    autoupdate::configure(&data, automatic, beta)?;
    if status::supervisor() == Some(status::Supervisor::Docker) {
        println!(
            "{}",
            tr!(
                "Dans l’image Docker, rien n’est remplacé : le serveur dit seulement qu’une nouvelle image existe.",
                "In the Docker image, nothing is replaced: the server only says that a new image exists."
            )
        );
    }
    println!("{}", autoupdate::shown(&data));
    Ok(())
}

/// `health`: the server of the folder runs and still writes its status (`status`): exit code 0, else 1 and why.
fn health(args: &Args) -> Result<()> {
    let data = data_of(args)?;
    let Some(s) = status::read(&data) else {
        bail!(tr!(
            "aucun serveur lancé sur {}",
            "no server running on {}",
            data.root().display()
        ));
    };
    status::healthy(&s, crate::model::now_ms(), status::alive).map_err(|e| anyhow!(e))?;
    println!(
        "{}",
        tr!(
            "escouade-server {} en marche (processus {}, API sur {})",
            "escouade-server {} running (process {}, API on {})",
            s.version,
            s.pid,
            s.api
        )
    );
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
fn run(args: &Args, web: Web) -> Result<()> {
    let page = browser_page(args.browser, web)?;
    let data = data_of(args)?;
    logs::install(data.root(), args.debug);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("escouade-server")
        .build()?;
    // What the core spawns runs on this runtime.
    tauri::async_runtime::set(runtime.handle().clone());
    let endpoint = Endpoint {
        listen: args.listen,
        port: args.port,
        hosts: args.hosts.clone(),
        proxies: args.proxies.clone(),
        gate: args.gate,
        page,
        private: crate::privateentry::Asked {
            gate: args.gate,
            listen: args.connector.listen,
            cert: args.connector.cert.clone(),
            key: args.connector.key.clone(),
            url: args.connector.url.clone(),
            proxies: args.proxies.clone(),
        },
    };
    let done = runtime.block_on(serve(
        data,
        endpoint,
        Limits::default(),
        |_| {},
        stop_asked(),
    ));
    runtime.shutdown_timeout(std::time::Duration::from_secs(5));
    done
}

/// The page `--browser` asks for (BC-1), none without it; refused when the binary carries none (a build without
/// `ESCOUADE_WEB_DIST`): `--browser` never starts a server that serves nothing.
pub fn browser_page(asked: bool, web: Web) -> Result<Option<Arc<Page>>> {
    if !asked {
        return Ok(None);
    }
    match Page::embedded(web.files)? {
        Some(page) => {
            log::debug!("escouade-server: the browser's page: {}", web.mark);
            Ok(Some(Arc::new(page)))
        }
        None => bail!(tr!(
            "--browser : ce escouade-server n’a pas la page du navigateur (construit sans ESCOUADE_WEB_DIST : npm run build:web, puis cargo build avec ESCOUADE_WEB_DIST=dist-web)",
            "--browser: this escouade-server carries no browser page (built without ESCOUADE_WEB_DIST: npm run build:web, then cargo build with ESCOUADE_WEB_DIST=dist-web)"
        )),
    }
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

/// Where the API listens, and the names it takes in a `Host` besides the loopback's.
#[derive(Debug, Clone, Default)]
pub struct Endpoint {
    /// Another address than 127.0.0.1, asked for in so many words (`--listen`).
    pub listen: Option<IpAddr>,
    pub port: u16,
    /// `--allow-host`.
    pub hosts: Vec<String>,
    /// `--trust-proxy`.
    pub proxies: Vec<IpAddr>,
    /// `--gate` (RP-1).
    pub gate: bool,
    /// The page of the browser's client (`--browser`, BC-1): none by default.
    pub page: Option<Arc<Page>>,
    /// The private network (RP-4): `--connector-listen` and its company (its `gate` is `--gate` too).
    pub private: crate::privateentry::Asked,
}

impl Endpoint {
    /// 127.0.0.1:`port`.
    pub fn port(port: u16) -> Self {
        Self {
            port,
            ..Self::default()
        }
    }
}

/// The server on `data`: the core started, the API served (`ready` is told it), until `stop` comes; then the API
/// closed and the core stopped as when the app quits, its state written. On a runtime of Tokio.
pub async fn serve(
    data: DataDir,
    endpoint: Endpoint,
    limits: Limits,
    ready: impl FnOnce(&Arc<RemoteApi>),
    stop: impl Future<Output = &'static str>,
) -> Result<()> {
    // The data folder first: nothing of it is touched while another process holds it.
    let lock = crate::headless::InstanceLock::acquire(data.root()).map_err(|e| anyhow!("{e:#}"))?;
    let store = Arc::new(secrets::FileStore::open(&data)?);
    let mut host = HeadlessHost::new(resources()).with_secrets(store.clone());
    // In a build of development only, another version (the e2e of « Mettre à jour le serveur », ESC-702).
    if let Some(v) = test_version() {
        host = host.with_version(&v);
    }
    let host = Arc::new(host);
    let root = data.root().to_path_buf();
    let listen = endpoint.listen;
    let port = endpoint.port;
    // The process groups of agents a killed server left behind, stopped before any agent starts (as the app).
    crate::agentgroups::start(root.join("agent-groups.json"));
    let headless = Headless::start_locked(lock, data, host);
    let core = headless.core.clone();
    // The teams of the team server (OE-1): a machine of the team in server mode is a device of its own, served to the
    // remote windows; `escouade-server team pair` (another process) leaves its request for it.
    let teams = headless.open_teams();
    teampair::watch(teams, root.clone());
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
    api.allow_hosts(endpoint.hosts.clone());
    api.trust_proxies(endpoint.proxies.clone());
    api.serve_page(endpoint.page.clone());
    // The phone's tunnel (ESC-567), when Réglages de l'espace › Téléphone turned it on: the same API, in the tunnel.
    core.phone.attach(&api);
    core.phone.mark_server();
    // Its Kanbans go up to 64 tickets at once (ESC-620).
    core.mark_server();
    // The private network (RP-1, RP-4), before any agent starts: the gate, the agents’ one proxy, and the connectors'
    // entry, whose channels the gate goes through; neither when a policy forbids it (`servers.privateNetwork`).
    let mut private = endpoint.private.clone();
    private.gate |= endpoint.gate;
    if private.proxies.is_empty() {
        private.proxies = endpoint.proxies.clone();
    }
    if private.any() {
        if let Err(e) = core.start_private_network(private, store.clone()).await {
            stop_core(headless).await;
            return Err(e);
        }
    }
    {
        let core = core.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = core.phone.apply(&core).await {
                log::warn!("escouade-server: the phone's tunnel is off: {e:#}");
            }
            // `escouade-server phone on|off|add|remove` from another process: its file read again, applied.
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                if core.phone.reload_if_changed() {
                    if let Err(e) = core.phone.apply(&core).await {
                        log::warn!("escouade-server: the phone's tunnel: {e:#}");
                    }
                }
            }
        });
    }
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
    // Its own update (ESC-702): its channel looked at, a version put in place, the restart at rest.
    let own_update = server_update(&core);
    if !endpoint.hosts.is_empty() {
        log::info!(
            "escouade-server: the API takes the Host {} besides the loopback's (--allow-host)",
            endpoint.hosts.join(", ")
        );
    }
    if let Some(page) = &endpoint.page {
        log::info!(
            "escouade-server: the browser's page on http://{shown}/ ({}; --browser): through a tunnel, or behind a proxy in https:// alone",
            page.summary
        );
    }
    if !endpoint.proxies.is_empty() {
        let proxies: Vec<String> = endpoint.proxies.iter().map(IpAddr::to_string).collect();
        log::info!(
            "escouade-server: X-Forwarded-For read from {} alone (--trust-proxy)",
            proxies.join(", ")
        );
    }
    // A new server: what to do to reach it, never the code itself (`pair` prints it, on its own output).
    if api.devices().list().is_empty() {
        log::info!("{}", first_start_hint(status::supervisor()));
    }
    // What it says of itself, for `health` and `update` beside it.
    let writer = tauri::async_runtime::spawn(write_status(core.clone(), shown.clone()));
    ready(&api);
    let why = match &own_update {
        Some(u) => tokio::select! {
            why = stop => why,
            _ = u.restarting() => "update",
        },
        None => stop.await,
    };
    log::info!("escouade-server: {why}: stopping (the agents' turns are cut, the state written)");
    writer.abort();
    core.private_network.close();
    core.gate.stop();
    let status_data = core.data.clone();
    drop(core);
    stop_core(headless).await;
    status::remove(&status_data);
    log::info!("escouade-server: stopped");
    Ok(())
}

/// In a build of development, `ESCOUADE_SERVER_TEST_VERSION`: the version the server says it is (never in a release).
fn test_version() -> Option<String> {
    if !cfg!(debug_assertions) {
        return None;
    }
    std::env::var("ESCOUADE_SERVER_TEST_VERSION")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| update::is_version(v))
}

/// The server's own update (ESC-702), kept in the core for the remote API and its commands, its loop started.
fn server_update(core: &Arc<crate::core::Core>) -> Option<Arc<autoupdate::ServerUpdate>> {
    let target = match autoupdate::Target::here() {
        Ok(t) => t,
        Err(e) => {
            log::warn!("escouade-server update: no automatic update: {e:#}");
            return None;
        }
    };
    let client = match update::client(&core.settings.read()) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("escouade-server update: no automatic update: {e:#}");
            return None;
        }
    };
    let source = autoupdate::Source {
        client,
        base: update::base(),
        key: update::key(),
        arch: update::arch().to_string(),
    };
    let u = autoupdate::ServerUpdate::new(&core.host.version(), core.data.clone(), source, target);
    // A build of development reads no real release by itself (as the app, ESC-543): a release of test, or nothing.
    let u = if cfg!(debug_assertions) && std::env::var(update::URL_VAR).is_err() {
        log::info!(
            "escouade-server update: a build of development looks for no version by itself without {}",
            update::URL_VAR
        );
        u.without_looking()
    } else {
        u
    };
    let _ = core.server_update.set(u.clone());
    u.watch(core.clone());
    Some(u)
}

/// What the log of a server with no paired device says: how to pair one (never a code).
pub fn first_start_hint(supervisor: Option<status::Supervisor>) -> String {
    let how = if supervisor == Some(status::Supervisor::Docker) {
        "docker exec <container> escouade-server pair"
    } else {
        "escouade-server pair"
    };
    tr!(
        "Aucun appareil appairé. Pour en appairer un : lance « {how} » sur ce serveur, puis donne le code qu’il affiche à Escouade sur ton poste (« Se connecter à un serveur »).",
        "No paired device. To pair one: run “{how}” on this server, then give the code it prints to Escouade on your computer (“Connect to a server”)."
    )
}

/// `server-status.json`, written again every `status::EVERY` and as soon as what would keep a restart waiting
/// changes (`updates::busy`, as the app's automatic restart reads it), until the task is stopped.
async fn write_status(core: Arc<crate::core::Core>, api: String) {
    let updates = crate::updates::Updates::default();
    let supervisor = status::supervisor();
    let started_at = crate::model::now_ms();
    let mut written: Option<(Option<String>, usize, Option<autoupdate::State>)> = None;
    let mut last = std::time::Instant::now();
    loop {
        let now = crate::model::now_ms();
        let snapshot = crate::updates::snapshot(&core, &updates, now);
        let busy = crate::updates::busy(&snapshot).map(|b| status::busy_word(b).to_string());
        let update = core.server_update.get().map(|u| u.state());
        let seen = (busy.clone(), snapshot.turns, update.clone());
        if written.as_ref() != Some(&seen) || last.elapsed() >= status::EVERY {
            let s = status::Status {
                pid: std::process::id(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                api: api.clone(),
                started_at,
                at: now,
                busy,
                turns: snapshot.turns,
                supervisor,
                update,
            };
            if let Err(e) = status::write(&core.data, &s) {
                log::warn!("escouade-server: {} not written: {e:#}", status::FILE);
            }
            written = Some(seen);
            last = std::time::Instant::now();
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}

/// The core stopped off the runtime's threads (its plugins are waited for): the API closed, the agents, terminals and
/// plugins stopped, the state written.
async fn stop_core(headless: Headless) {
    let stopped = tokio::task::spawn_blocking(move || headless.stop()).await;
    if let Err(e) = stopped {
        log::error!("escouade-server: the stop did not finish: {e}");
    }
}
