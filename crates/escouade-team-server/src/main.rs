//! `escouade-team-server init` creates the base and prints the one-use token that creates the team;
//! `escouade-team-server serve` serves. Each option also reads an environment variable (`--help`).

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use escouade_team_server::config::{Config, DatabaseConfig, FilesConfig, S3Config};
use escouade_team_server::{transport, Server};

const HELP: &str = "\
escouade-team-server: Escouade's team server. It keeps only encrypted bytes and runs nothing.

USAGE:
    escouade-team-server init  [OPTIONS]   create the base, print the token that creates the team
    escouade-team-server serve [OPTIONS]   serve
    escouade-team-server recovery-token --team <ID> [OPTIONS]
                                           print a one-use token for the holder of a team's recovery key, the
                                           day every administrator lost their devices: it enrols one device as
                                           an administrator of the server (the 24 words give the keys, not it)

OPTIONS (and their environment variables):
    --data <DIR>              ESCOUADE_TEAM_DATA            the folder of team.db and files/ [./data]
    --database <URL>          ESCOUADE_TEAM_DATABASE_URL    postgres://… instead of SQLite
    --files <DIR>             ESCOUADE_TEAM_FILES           the files' folder [<data>/files]
    --s3-endpoint <URL>       ESCOUADE_TEAM_S3_ENDPOINT     the files in S3 (MinIO, Garage, AWS), path-style
    --s3-bucket <NAME>        ESCOUADE_TEAM_S3_BUCKET
    --s3-region <NAME>        ESCOUADE_TEAM_S3_REGION       [us-east-1]
    --s3-prefix <TEXT>        ESCOUADE_TEAM_S3_PREFIX       []
                              ESCOUADE_TEAM_S3_ACCESS_KEY, ESCOUADE_TEAM_S3_SECRET_KEY (environment only)
    --listen <ADDR>           ESCOUADE_TEAM_LISTEN          [127.0.0.1:8443]
    --tls-cert <PEM>          ESCOUADE_TEAM_TLS_CERT        TLS by rustls; without, put a reverse proxy in front
    --tls-key <PEM>           ESCOUADE_TEAM_TLS_KEY
    --trust-proxy             ESCOUADE_TEAM_TRUST_PROXY=1   the client is the last hop of X-Forwarded-For
    --metrics-listen <ADDR>   ESCOUADE_TEAM_METRICS_LISTEN  Prometheus on /metrics, on its own address [off]
    --max-file <BYTES>        ESCOUADE_TEAM_MAX_FILE        [26214400]
    --team-quota <BYTES>      ESCOUADE_TEAM_QUOTA           [2147483648]
    RUST_LOG                  the log's level [info]
";

struct Args {
    command: String,
    flags: Vec<(String, Option<String>)>,
}

const SWITCHES: &[&str] = &["--trust-proxy", "--help", "-h"];

fn parse() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let command = it.next().unwrap_or_else(|| "--help".into());
    let mut flags = Vec::new();
    while let Some(a) = it.next() {
        if !a.starts_with('-') {
            return Err(format!("unexpected: {a}"));
        }
        if let Some((k, v)) = a.split_once('=') {
            flags.push((k.to_string(), Some(v.to_string())));
        } else if SWITCHES.contains(&a.as_str()) {
            flags.push((a, None));
        } else {
            let v = it.next().ok_or_else(|| format!("{a} needs a value"))?;
            flags.push((a, Some(v)));
        }
    }
    Ok(Args { command, flags })
}

impl Args {
    fn get(&self, flag: &str, env: &str) -> Option<String> {
        self.flags
            .iter()
            .rev()
            .find(|(k, _)| k == flag)
            .and_then(|(_, v)| v.clone())
            .or_else(|| std::env::var(env).ok().filter(|v| !v.is_empty()))
    }

    fn has(&self, flag: &str, env: &str) -> bool {
        self.flags.iter().any(|(k, _)| k == flag)
            || std::env::var(env).is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
    }

    fn number(&self, flag: &str, env: &str) -> Result<Option<u64>, String> {
        self.get(flag, env)
            .map(|v| {
                v.parse::<u64>()
                    .map_err(|_| format!("{flag}: not a number"))
            })
            .transpose()
    }

    fn config(&self) -> Result<Config, String> {
        let data = PathBuf::from(
            self.get("--data", "ESCOUADE_TEAM_DATA")
                .unwrap_or_else(|| "data".into()),
        );
        let mut config = Config::in_folder(&data);
        if let Some(url) = self.get("--database", "ESCOUADE_TEAM_DATABASE_URL") {
            if !url.starts_with("postgres://") && !url.starts_with("postgresql://") {
                return Err(
                    "--database takes a postgres:// address (SQLite is the default, in --data)"
                        .into(),
                );
            }
            config.database = DatabaseConfig::Postgres { url, schema: None };
        }
        if let Some(dir) = self.get("--files", "ESCOUADE_TEAM_FILES") {
            config.files = FilesConfig::Disk(dir.into());
        }
        if let Some(endpoint) = self.get("--s3-endpoint", "ESCOUADE_TEAM_S3_ENDPOINT") {
            let need =
                |name: &str| std::env::var(name).map_err(|_| format!("{name} is needed with S3"));
            config.files = FilesConfig::S3(S3Config {
                endpoint,
                bucket: self
                    .get("--s3-bucket", "ESCOUADE_TEAM_S3_BUCKET")
                    .ok_or("--s3-bucket is needed with S3")?,
                region: self
                    .get("--s3-region", "ESCOUADE_TEAM_S3_REGION")
                    .unwrap_or_else(|| "us-east-1".into()),
                prefix: self
                    .get("--s3-prefix", "ESCOUADE_TEAM_S3_PREFIX")
                    .unwrap_or_default(),
                access_key: need("ESCOUADE_TEAM_S3_ACCESS_KEY")?,
                secret_key: need("ESCOUADE_TEAM_S3_SECRET_KEY")?,
            });
        }
        if let Some(n) = self.number("--max-file", "ESCOUADE_TEAM_MAX_FILE")? {
            config.limits.max_file = n as usize;
        }
        if let Some(n) = self.number("--team-quota", "ESCOUADE_TEAM_QUOTA")? {
            config.limits.team_quota = n;
        }
        config.trust_proxy = self.has("--trust-proxy", "ESCOUADE_TEAM_TRUST_PROXY");
        Ok(config)
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
}

async fn run(args: Args) -> Result<(), String> {
    match args.command.as_str() {
        "init" => {
            let server = Server::open(args.config()?)
                .await
                .map_err(|e| e.to_string())?;
            let token = server.new_team_token().await.map_err(|e| e.to_string())?;
            println!("The base is ready.");
            println!();
            println!("The token that creates the team (one use, keep it secret until then):");
            println!();
            println!("    {token}");
            println!();
            println!("The first administrator gives it to Escouade with the server's address. The base and the files");
            println!("hold only encrypted bytes: back them up, but a backup is useless without the members' keys.");
            Ok(())
        }
        "recovery-token" => {
            let team = args
                .get("--team", "ESCOUADE_TEAM_ID")
                .and_then(|t| escouade_team_server::util::id_from_hex(t.trim()))
                .ok_or(
                    "--team takes the team's id (32 hexadecimal digits, as Escouade shows it)",
                )?;
            let server = Server::open(args.config()?)
                .await
                .map_err(|e| e.to_string())?;
            let token = server
                .new_recovery_token(team)
                .await
                .map_err(|e| e.to_string())?;
            println!(
                "The token for the holder of the team's recovery key (one use, 7 days at most):"
            );
            println!();
            println!("    {token}");
            println!();
            println!("Escouade asks for it with the server's address and the 24 words, in « Récupérer une équipe ».");
            Ok(())
        }
        "serve" => {
            let listen: SocketAddr = args
                .get("--listen", "ESCOUADE_TEAM_LISTEN")
                .unwrap_or_else(|| "127.0.0.1:8443".into())
                .parse()
                .map_err(|_| "--listen takes an address and a port (0.0.0.0:8443)")?;
            let tls = match (
                args.get("--tls-cert", "ESCOUADE_TEAM_TLS_CERT"),
                args.get("--tls-key", "ESCOUADE_TEAM_TLS_KEY"),
            ) {
                (Some(cert), Some(key)) => Some(
                    transport::listen::tls_config(cert.as_ref(), key.as_ref())
                        .map_err(|e| format!("TLS: {e}"))?,
                ),
                (None, None) => None,
                _ => return Err("--tls-cert and --tls-key go together".into()),
            };
            let server = Server::open(args.config()?)
                .await
                .map_err(|e| e.to_string())?;
            server.spawn_sweeper();
            if let Some(addr) = args.get("--metrics-listen", "ESCOUADE_TEAM_METRICS_LISTEN") {
                let addr: SocketAddr = addr
                    .parse()
                    .map_err(|_| "--metrics-listen takes an address and a port")?;
                let listener = tokio::net::TcpListener::bind(addr)
                    .await
                    .map_err(|e| format!("{addr}: {e}"))?;
                let router = server.metrics_router();
                tokio::spawn(async move {
                    let _ = axum::serve(listener, router).await;
                });
                tracing::info!(%addr, "metrics");
            }
            let listener = tokio::net::TcpListener::bind(listen)
                .await
                .map_err(|e| format!("{listen}: {e}"))?;
            tracing::info!(addr = %listen, tls = tls.is_some(), "serving");
            server
                .serve(listener, tls, shutdown_signal())
                .await
                .map_err(|e| e.to_string())
        }
        "--help" | "-h" | "help" => {
            print!("{HELP}");
            Ok(())
        }
        other => Err(format!(
            "unknown command: {other} (init, serve, recovery-token, --help)"
        )),
    }
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_target(false)
        .init();
    let args = match parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}\n\n{HELP}");
            return ExitCode::from(2);
        }
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("a runtime");
    match runtime.block_on(run(args)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("escouade-team-server: {e}");
            ExitCode::FAILURE
        }
    }
}
