//! `escouade-connector [--config <file>] [--state <folder>] <pair <address> <code> | unpair | run | check | version>`

use std::io::BufRead;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use escouade_connector::commands;
use escouade_connector::config;
use escouade_connector::guard::{Guards, SystemResolver, CONNECT_TIMEOUT};
use escouade_connector::journal::Journal;
use escouade_connector::keys::{default_state_dir, State};
use escouade_connector::run::{self, Options, Wss, RELOAD_EVERY};
use zeroize::Zeroizing;

const USAGE: &str = "escouade-connector, the connector of a private network for escouade-server

Usage: escouade-connector [--config <file>] [--state <folder>] <command>

Commands:
  pair <address> <code>  pair with the server (the address and the code from Space settings › Private network;
                         « - » as the code reads it from the standard input)
  unpair                 forget the keys
  run                    the service: the channel to the server, the journal in JSON on the standard output
  check                  check that each host of the ceiling resolves and answers
  version                this program's version

Options:
  --config <file>   connector.toml (ESCOUADE_CONNECTOR_CONFIG; by default /etc/escouade-connector/connector.toml,
                    %ProgramData%\\escouade-connector\\connector.toml under Windows)
  --state <folder>  where the keys are kept ($STATE_DIRECTORY; by default /var/lib/escouade-connector,
                    %ProgramData%\\escouade-connector under Windows)";

fn default_config() -> PathBuf {
    if let Some(p) = std::env::var_os("ESCOUADE_CONNECTOR_CONFIG").filter(|p| !p.is_empty()) {
        return PathBuf::from(p);
    }
    if cfg!(windows) {
        let base = std::env::var_os("ProgramData").unwrap_or_else(|| "C:\\ProgramData".into());
        PathBuf::from(base)
            .join("escouade-connector")
            .join("connector.toml")
    } else {
        PathBuf::from("/etc/escouade-connector/connector.toml")
    }
}

fn fail(message: impl std::fmt::Display) -> ExitCode {
    eprintln!("escouade-connector: {message}");
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut config_path = default_config();
    let mut state_dir = default_state_dir();
    let mut rest = Vec::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => match args.next() {
                Some(p) => config_path = PathBuf::from(p),
                None => return fail("--config wants a file"),
            },
            "--state" => match args.next() {
                Some(p) => state_dir = PathBuf::from(p),
                None => return fail("--state wants a folder"),
            },
            "-h" | "--help" | "help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "-V" | "--version" => rest.push("version".to_string()),
            _ => rest.push(arg),
        }
    }
    let state = State::new(state_dir);
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => return fail(e),
    };
    let version = escouade_connector::version();
    match rest.first().map(String::as_str) {
        Some("version") if rest.len() == 1 => {
            println!("escouade-connector {version}");
            ExitCode::SUCCESS
        }
        Some("pair") if rest.len() == 3 => {
            let code = Zeroizing::new(if rest[2] == "-" {
                let mut line = String::new();
                if std::io::stdin().lock().read_line(&mut line).is_err() {
                    return fail("no code on the standard input");
                }
                line.trim().to_string()
            } else {
                rest[2].clone()
            });
            let config = if config_path.exists() {
                match config::load(&config_path) {
                    Ok((c, _)) => Some(c),
                    Err(e) => return fail(e),
                }
            } else {
                None
            };
            match runtime.block_on(commands::pair(
                &rest[1],
                &code,
                config.as_ref(),
                &state,
                version,
                true,
                &Wss,
            )) {
                Ok(said) => {
                    println!("{said}");
                    ExitCode::SUCCESS
                }
                Err(e) => fail(e),
            }
        }
        Some("unpair") if rest.len() == 1 => match commands::unpair(&state) {
            Ok(said) => {
                println!("{said}");
                ExitCode::SUCCESS
            }
            Err(e) => fail(e),
        },
        Some("check") if rest.len() == 1 => {
            let config = match config::load(&config_path) {
                Ok((c, _)) => c,
                Err(e) => return fail(e),
            };
            println!("connector.toml: {}", config_path.display());
            match state.load() {
                Ok(Some((p, _))) => println!("paired with {}", p.server),
                Ok(None) => println!("not paired (escouade-connector pair <address> <code>)"),
                Err(e) => println!("the pairing does not read: {e}"),
            }
            if config.ceiling.rules.is_empty() {
                println!("the ceiling is empty: nothing passes");
            }
            let lines = runtime.block_on(commands::check(
                &config,
                &SystemResolver,
                &Guards::default(),
                CONNECT_TIMEOUT,
            ));
            let failed = lines.iter().filter(|l| !l.ok).count();
            for line in &lines {
                println!(
                    "{} {}: {}",
                    if line.ok { "ok  " } else { "FAIL" },
                    line.rule,
                    line.text
                );
            }
            if failed == 0 {
                ExitCode::SUCCESS
            } else {
                eprintln!("escouade-connector: {failed} host(s) of the ceiling do not answer");
                ExitCode::FAILURE
            }
        }
        Some("run") if rest.len() == 1 => {
            let opts = Options {
                config_path,
                state,
                journal: Journal::stdout(),
                resolver: Arc::new(SystemResolver),
                guards: Guards::default(),
                version,
                system_roots: true,
                reload_every: RELOAD_EVERY,
            };
            let (stop_tx, stop) = tokio::sync::watch::channel(false);
            // Ctrl+C, and under Unix SIGTERM (what systemd and a container send to stop).
            runtime.spawn(async move {
                #[cfg(unix)]
                {
                    use tokio::signal::unix::{signal, SignalKind};
                    match signal(SignalKind::terminate()) {
                        Ok(mut term) => {
                            tokio::select! {
                                _ = tokio::signal::ctrl_c() => {}
                                _ = term.recv() => {}
                            }
                        }
                        Err(_) => {
                            let _ = tokio::signal::ctrl_c().await;
                        }
                    }
                }
                #[cfg(not(unix))]
                {
                    let _ = tokio::signal::ctrl_c().await;
                }
                let _ = stop_tx.send(true);
            });
            match runtime.block_on(run::run(opts, Wss, stop)) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => fail(e),
            }
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
