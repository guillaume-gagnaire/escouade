//! `escouade-team-server init` creates the base and prints the one-use token that creates the team;
//! `escouade-team-server serve` serves. Each option also reads an environment variable (`--help`).

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use escouade_team_server::config::{Config, DatabaseConfig, FilesConfig, S3Config};
use escouade_team_server::db::{entry_digest, Kind, Page, Stream};
use escouade_team_server::{transport, Server};

macro_rules! storage_options {
    () => {
        "    --data <DIR>              ESCOUADE_TEAM_DATA            the folder of team.db and files/ [./data]
    --database <URL>          ESCOUADE_TEAM_DATABASE_URL    postgres://… instead of SQLite
    --files <DIR>             ESCOUADE_TEAM_FILES           the files' folder [<data>/files]
    --s3-endpoint <URL>       ESCOUADE_TEAM_S3_ENDPOINT     the files in S3 (MinIO, Garage, AWS), path-style
    --s3-bucket <NAME>        ESCOUADE_TEAM_S3_BUCKET
    --s3-region <NAME>        ESCOUADE_TEAM_S3_REGION       [us-east-1]
    --s3-prefix <TEXT>        ESCOUADE_TEAM_S3_PREFIX       []
                              ESCOUADE_TEAM_S3_ACCESS_KEY, ESCOUADE_TEAM_S3_SECRET_KEY (environment only)
    --max-file <BYTES>        ESCOUADE_TEAM_MAX_FILE        [26214400]
    --team-quota <BYTES>      ESCOUADE_TEAM_QUOTA           [2147483648]
"
    };
}

macro_rules! serve_options {
    () => {
        "    --listen <ADDR>           ESCOUADE_TEAM_LISTEN          [127.0.0.1:8443]
    --tls-cert <PEM>          ESCOUADE_TEAM_TLS_CERT        TLS by rustls; without, put a reverse proxy in front
    --tls-key <PEM>           ESCOUADE_TEAM_TLS_KEY
    --trust-proxy             ESCOUADE_TEAM_TRUST_PROXY=1   the client is the last hop of X-Forwarded-For
    --metrics-listen <ADDR>   ESCOUADE_TEAM_METRICS_LISTEN  Prometheus on /metrics, on its own address [off]
"
    };
}

macro_rules! tail_options {
    () => {
        "    RUST_LOG                  the log's level [info]
    -h, --help                print this help and exit: nothing is created, nothing listens
    -V, --version             print the version and exit
"
    };
}

const HELP: &str = concat!(
    "\
escouade-team-server: Escouade's team server. It keeps only encrypted bytes and runs nothing.

USAGE:
    escouade-team-server init  [OPTIONS]   create the base, print the token that creates the team
    escouade-team-server serve [OPTIONS]   serve
    escouade-team-server recovery-token --team <ID> [OPTIONS]
                                           print a one-use token for the holder of a team's recovery key, the
                                           day every administrator lost their devices: it enrols one device as
                                           an administrator of the server (the 24 words give the keys, not it)
    escouade-team-server drop-entry --team <ID> (--stream <NAME> | --project <ID>) --seq <N> [--yes]
                                           remove one entry that blocks a team or a project (a commit the
                                           devices cannot read past), after a confirmation; the base keeps the
                                           line of the audit (who, when, what, its digest), never its bytes
    escouade-team-server help [COMMAND]    this help, or a command's

`escouade-team-server <COMMAND> --help` lists the options of one command.

OPTIONS (and their environment variables):
",
    storage_options!(),
    serve_options!(),
    "    --team <ID>               ESCOUADE_TEAM_ID              recovery-token, drop-entry: the team's id\n",
    tail_options!(),
);

const HELP_INIT: &str = concat!(
    "\
escouade-team-server init: create the base (and the files' folder) if they do not exist, then print the
one-use token that creates a team. Run again, it prints another token, for another team on the same server.

USAGE:
    escouade-team-server init [OPTIONS]

OPTIONS (and their environment variables):
",
    storage_options!(),
    tail_options!(),
);

const HELP_SERVE: &str = concat!(
    "\
escouade-team-server serve: serve the team server until it is stopped (Ctrl-C or SIGTERM).

USAGE:
    escouade-team-server serve [OPTIONS]

OPTIONS (and their environment variables):
",
    storage_options!(),
    serve_options!(),
    tail_options!(),
);

const HELP_RECOVERY: &str = concat!(
    "\
escouade-team-server recovery-token: print a one-use token (7 days at most) for the holder of a team's
recovery key, the day every administrator lost their devices: it enrols one device as an administrator of
the server (the 24 words give the keys, not it).

USAGE:
    escouade-team-server recovery-token --team <ID> [OPTIONS]

OPTIONS (and their environment variables):
    --team <ID>               ESCOUADE_TEAM_ID              the team's id (32 hexadecimal digits) [required]
",
    storage_options!(),
    tail_options!(),
);

const HELP_DROP: &str = concat!(
    "\
escouade-team-server drop-entry: remove one entry of a team's stream or of a project's, the day an entry blocks it
(a commit no device can read past: the team, or the project, no longer moves on). It shows the entry, asks for a
confirmation (or --yes), removes it and keeps in the base the line of the audit: who (--by), when, which stream,
which number, its kind and epoch, its digest (SHA-256), never its bytes. The stream keeps its numbering: the devices
see the hole, tell their administrators, and read on. A commit must be the last of its stream (remove the later
ones first): its epoch is then free again for a commit the devices can read. It neither reads an entry nor brings
one back: the server holds only encrypted bytes. Never remove a commit the devices already read.

USAGE:
    escouade-team-server drop-entry --team <ID> --stream team --seq <N> [OPTIONS]
    escouade-team-server drop-entry --team <ID> --project <ID> --seq <N> [OPTIONS]
    escouade-team-server drop-entry --team <ID> --list [OPTIONS]

OPTIONS (and their environment variables):
    --team <ID>               ESCOUADE_TEAM_ID              the team's id (32 hexadecimal digits) [required]
    --stream <NAME>                                         team (the team's stream), or one of the team's own
                                                            streams (escouade/project, escouade/setting, ...)
    --project <ID>                                          a project's stream (32 hexadecimal digits)
    --seq <N>                                               the number of the entry to remove [required]
    --by <NAME>               ESCOUADE_TEAM_OPERATOR        who removes it, for the audit [the account's name]
    --yes                                                   remove it without asking
    --list                                                  print the base's audit of what was removed from the
                                                            team, and remove nothing
",
    storage_options!(),
    tail_options!(),
);

struct Args {
    command: String,
    flags: Vec<(String, Option<String>)>,
}

/// What the command line asks for.
enum Action {
    /// Print this text on stdout and exit 0, having done nothing else.
    Print(String),
    Run(Args),
}

/// The options that take a value, for every command that opens the base.
const STORAGE: &[&str] = &[
    "--data",
    "--database",
    "--files",
    "--s3-endpoint",
    "--s3-bucket",
    "--s3-region",
    "--s3-prefix",
    "--max-file",
    "--team-quota",
];
const SERVE: &[&str] = &["--listen", "--tls-cert", "--tls-key", "--metrics-listen"];

fn command_help(command: &str) -> Option<&'static str> {
    match command {
        "init" => Some(HELP_INIT),
        "serve" => Some(HELP_SERVE),
        "recovery-token" => Some(HELP_RECOVERY),
        "drop-entry" => Some(HELP_DROP),
        _ => None,
    }
}

/// The options a command takes: (with a value, switches).
fn command_options(command: &str) -> (Vec<&'static str>, &'static [&'static str]) {
    let mut valued = STORAGE.to_vec();
    let mut switches: &[&str] = &[];
    match command {
        "serve" => {
            valued.extend(SERVE);
            switches = &["--trust-proxy"];
        }
        "recovery-token" => valued.push("--team"),
        "drop-entry" => {
            valued.extend(["--team", "--stream", "--project", "--seq", "--by"]);
            switches = &["--yes", "--list"];
        }
        _ => {}
    }
    (valued, switches)
}

fn version_line() -> String {
    format!("escouade-team-server {}\n", env!("CARGO_PKG_VERSION"))
}

fn parse_from(argv: Vec<String>) -> Result<Action, String> {
    let mut it = argv.into_iter();
    let command = it.next().unwrap_or_else(|| "--help".into());
    match command.as_str() {
        "--help" | "-h" => return Ok(Action::Print(HELP.into())),
        "--version" | "-V" => return Ok(Action::Print(version_line())),
        "help" => {
            return match it.next() {
                None => Ok(Action::Print(HELP.into())),
                Some(c) => match command_help(&c) {
                    Some(h) if it.next().is_none() => Ok(Action::Print(h.into())),
                    Some(_) => Err("help takes one command at most".into()),
                    None => Err(format!(
                        "unknown command: {c} (init, serve, recovery-token, drop-entry)"
                    )),
                },
            }
        }
        _ => {}
    }
    let Some(help) = command_help(&command) else {
        return Err(format!(
            "unknown command: {command} (init, serve, recovery-token, drop-entry, help)"
        ));
    };
    let rest: Vec<String> = it.collect();
    // Help and version win over everything, so asking for them never runs the command.
    if rest.iter().any(|a| a == "--help" || a == "-h") {
        return Ok(Action::Print(help.into()));
    }
    if rest.iter().any(|a| a == "--version" || a == "-V") {
        return Ok(Action::Print(version_line()));
    }
    let (valued, switches) = command_options(&command);
    let mut flags = Vec::new();
    let mut it = rest.into_iter();
    while let Some(a) = it.next() {
        if !a.starts_with('-') {
            return Err(format!("unexpected argument for {command}: {a}"));
        }
        let (name, inline) = match a.split_once('=') {
            Some((k, v)) => (k.to_string(), Some(v.to_string())),
            None => (a.clone(), None),
        };
        if switches.contains(&name.as_str()) {
            if inline.is_some() {
                return Err(format!("{name} takes no value"));
            }
            flags.push((name, None));
        } else if valued.contains(&name.as_str()) {
            let v = match inline {
                Some(v) => v,
                None => it.next().ok_or_else(|| format!("{name} needs a value"))?,
            };
            flags.push((name, Some(v)));
        } else {
            return Err(format!("unknown option for {command}: {name}"));
        }
    }
    Ok(Action::Run(Args { command, flags }))
}

fn parse() -> Result<Action, String> {
    parse_from(std::env::args().skip(1).collect())
}

impl Args {
    fn get(&self, flag: &str, env: &str) -> Option<String> {
        self.flags
            .iter()
            .rev()
            .find(|(k, _)| k == flag)
            .and_then(|(_, v)| v.clone())
            .or_else(|| {
                // An option without its variable (`""`) reads none.
                (!env.is_empty())
                    .then(|| std::env::var(env).ok())
                    .flatten()
                    .filter(|v| !v.is_empty())
            })
    }

    fn has(&self, flag: &str, env: &str) -> bool {
        self.flags.iter().any(|(k, _)| k == flag)
            || !env.is_empty()
                && std::env::var(env).is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
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
        "drop-entry" => drop_entry(&args).await,
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
        other => Err(format!("unknown command: {other}")),
    }
}

/// The stream `drop-entry` names: the team's, one of the team's own (by its name), or a project's (by its id).
fn dropped_stream(args: &Args) -> Result<(Stream, String), String> {
    let names = || {
        escouade_team_server::api::TEAM_STREAMS
            .iter()
            .map(|id| String::from_utf8_lossy(id).into_owned())
            .collect::<Vec<_>>()
            .join(", ")
    };
    match (args.get("--stream", ""), args.get("--project", "")) {
        (Some(_), Some(_)) => {
            Err("--stream and --project name one stream: give one of them only".into())
        }
        (None, None) => Err(
            "drop-entry needs the stream: --stream team, --stream <name> or --project <ID>".into(),
        ),
        (None, Some(p)) => {
            let id = escouade_team_server::util::id_from_hex(p.trim())
                .ok_or("--project takes the project's id (32 hexadecimal digits)")?;
            Ok((Stream::Project(id), format!("project {}", hex::encode(id))))
        }
        (Some(s), None) if s == "team" => Ok((Stream::Team, "team".into())),
        (Some(s), None) => {
            let own = escouade_team_server::api::TEAM_STREAMS
                .iter()
                .find(|id| id.as_slice() == s.as_bytes())
                .ok_or_else(|| {
                    format!(
                        "--stream takes team or one of the team's own streams ({})",
                        names()
                    )
                })?;
            Ok((Stream::Project(*own), s))
        }
    }
}

/// Who removes an entry, for the audit: `--by`, else the account's name.
fn operator(args: &Args) -> String {
    args.get("--by", "ESCOUADE_TEAM_OPERATOR")
        .or_else(|| std::env::var("USER").ok())
        .or_else(|| std::env::var("USERNAME").ok())
        .map(|n| {
            n.trim()
                .chars()
                .filter(|c| !c.is_control())
                .take(80)
                .collect::<String>()
        })
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "unknown".into())
}

/// `drop-entry`: shows the entry, asks, removes it with its line of audit.
async fn drop_entry(args: &Args) -> Result<(), String> {
    let team = args
        .get("--team", "ESCOUADE_TEAM_ID")
        .and_then(|t| escouade_team_server::util::id_from_hex(t.trim()))
        .ok_or(
            "--team takes the team's id (32 hexadecimal digits, as Escouade shows it): drop-entry never guesses it",
        )?;
    if args.has("--list", "") {
        return list_dropped(args, team).await;
    }
    let (stream, label) = dropped_stream(args)?;
    let seq = args
        .number("--seq", "")?
        .filter(|n| *n > 0)
        .ok_or("--seq takes the number of the entry to remove (1 or more)")?;
    let by = operator(args);
    let server = Server::open(args.config()?)
        .await
        .map_err(|e| e.to_string())?;
    let store = &server.app().store;
    if store
        .devices(team)
        .await
        .map_err(|e| e.to_string())?
        .is_empty()
    {
        return Err(format!("the base holds no team {}", hex::encode(team)));
    }
    let page = Page {
        entries: 1,
        bytes: usize::MAX,
    };
    let (entries, _) = store
        .read(team, stream, seq - 1, page)
        .await
        .map_err(|e| e.to_string())?;
    let Some(entry) = entries.into_iter().next().filter(|e| e.seq == seq) else {
        return Err(format!(
            "the {label} stream of this team holds no entry {seq}: nothing removed"
        ));
    };
    let what = match (entry.kind, entry.epoch) {
        (Kind::Commit, Some(epoch)) => format!("a commit, for epoch {epoch}"),
        (Kind::Commit, None) => "a commit".into(),
        (Kind::Message, _) => "a message".into(),
        (Kind::Void, Some(epoch)) => format!("the void of the commit of epoch {epoch}"),
        (Kind::Void, None) => "a void".into(),
    };
    println!(
        "Team {}, {label} stream, entry {seq}: {what}.",
        hex::encode(team)
    );
    println!(
        "    written by the device {} at {} (seconds since 1970), {} bytes",
        hex::encode(&entry.device),
        entry.at,
        entry.payload.len()
    );
    println!(
        "    its digest (SHA-256) {}",
        hex::encode(entry_digest(&entry))
    );
    println!();
    if !args.has("--yes", "") {
        use std::io::{BufRead, Write};
        print!("Remove it? The devices will see the hole; nothing brings the entry back. Type yes to remove it: ");
        let _ = std::io::stdout().flush();
        let mut answer = String::new();
        let _ = std::io::stdin().lock().read_line(&mut answer);
        println!();
        if answer.trim() != "yes" {
            return Err("not confirmed: nothing removed".into());
        }
    }
    let dropped = store
        .drop_entry(
            team,
            stream,
            seq,
            by.clone(),
            escouade_team_server::util::now(),
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or("the entry went meanwhile: nothing removed")?;
    // The log keeps no id (the guide says so); the base's audit holds the team, the stream and the digest.
    tracing::info!(
        target: "audit",
        action = "drop-entry",
        seq,
        kind = ?dropped.kind,
        by = %by,
        "an entry removed by the operator"
    );
    println!("Removed, and noted in the base's audit (by {by}). The devices read on past it at their next reading.");
    Ok(())
}

/// `drop-entry --list`: the base's audit of the entries removed from a team's streams, the oldest first.
async fn list_dropped(args: &Args, team: escouade_team_server::util::Id) -> Result<(), String> {
    let server = Server::open(args.config()?)
        .await
        .map_err(|e| e.to_string())?;
    let audit = server
        .app()
        .store
        .dropped(team)
        .await
        .map_err(|e| e.to_string())?;
    if audit.is_empty() {
        println!("No entry of this team was removed.");
    }
    for d in audit {
        let stream = match d.project {
            None => "team".to_string(),
            Some(p) if escouade_team_server::api::TEAM_STREAMS.contains(&p) => {
                String::from_utf8_lossy(&p).into_owned()
            }
            Some(p) => format!("project {}", hex::encode(p)),
        };
        let what = match (d.kind, d.epoch) {
            (Kind::Commit, Some(e)) => format!("commit, epoch {e}"),
            (Kind::Commit, None) => "commit".into(),
            (Kind::Message, _) => "message".into(),
            (Kind::Void, Some(e)) => format!("void, epoch {e}"),
            (Kind::Void, None) => "void".into(),
        };
        println!(
            "{} by {}: {stream} stream, entry {} ({what}, {} bytes, written by {} at {}), digest {}",
            d.at,
            d.operator,
            d.seq,
            d.size,
            hex::encode(d.device),
            d.written_at,
            hex::encode(d.digest)
        );
    }
    Ok(())
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_target(false)
        .init();
    let args = match parse() {
        Ok(Action::Print(text)) => {
            print!("{text}");
            return ExitCode::SUCCESS;
        }
        Ok(Action::Run(a)) => a,
        Err(e) => {
            eprintln!("escouade-team-server: {e}");
            eprintln!("Usage error: `escouade-team-server --help` lists the commands, `escouade-team-server <command> --help` the options of one.");
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
