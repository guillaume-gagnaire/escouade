# Install the team server

For whoever hosts a team’s server: build it, run it with Docker or systemd, create the team, choose the database and the storage, put it behind TLS, back it up, update it and monitor it.

> [!WARNING]
> The team server is a preview of 2.3. Nothing is published yet: no binary, no image. You build it from Escouade’s repository. Signed binaries and a published image are coming.

## What the server does

`escouade-team-server` keeps, numbers and broadcasts what the team’s devices send it, end-to-end encrypted. It holds no content key and runs nothing: no agent, no command, no plugin. What it sees is detailed in [the security officer’s guide](team/security#what-the-server-sees).

It is a program apart from Escouade’s server mode, which runs agents and is never exposed. The team server, on the other hand, must be reachable by every member, over HTTPS.

## Requirements

- A Linux machine every member can reach, and a domain name pointed at it (`team.example.org`).
- For the example compose file: Docker with Compose, and ports 80 and 443 open (Caddy gets the certificate).
- For the binary: the repository’s Rust toolchain (`src-tauri/rust-toolchain.toml`).
- Optionally, a PostgreSQL database and S3-compatible storage (MinIO, Garage, AWS).
- On the members’ side: Escouade reaches a server over `https://`; over `http://` only on the machine itself (`127.0.0.1`, `localhost`), for a trial.

## Build the binary

From Escouade’s repository:

```sh
cd src-tauri && cargo build --release -p escouade-team-server
# the binary: src-tauri/target/release/escouade-team-server
```

`escouade-team-server --help`, or the program run with no argument, lists the commands and the options.

> [!WARNING]
> The subcommands don’t read `--help`: `escouade-team-server init --help` runs a real `init` in `./data`. Ask for help without a subcommand.

## The commands

| Command | What it does |
|---|---|
| `escouade-team-server init` | Creates the database if it doesn’t exist, then prints a one-use token that creates a team. Run again, it gives another token, for another team on the same server. |
| `escouade-team-server serve` | Serves, until `Ctrl+C` or the service stops. |
| `escouade-team-server recovery-token --team <id>` | Prints a one-use token, valid for 7 days at most, for the holder of the recovery key of a team whose administrators all lost their devices (see [Recover a team](#recover-a-team)). |

All three take the same database and file options: give `init` and `recovery-token` the same ones as `serve`.

## The options

Each option has its environment variable; the option wins over the variable. An option is written `--listen 0.0.0.0:8443` or `--listen=0.0.0.0:8443`. An empty variable counts as unset.

| Option | Variable | Default | What it sets |
|---|---|---|---|
| `--data <folder>` | `ESCOUADE_TEAM_DATA` | `./data` | The folder of the SQLite database (`team.db`) and of the files (`files/`). |
| `--database <address>` | `ESCOUADE_TEAM_DATABASE_URL` | SQLite in `--data` | A PostgreSQL database: `postgres://…` or `postgresql://…`. |
| `--files <folder>` | `ESCOUADE_TEAM_FILES` | `<data>/files` | The folder of the encrypted files. |
| `--s3-endpoint <address>` | `ESCOUADE_TEAM_S3_ENDPOINT` | none | The files in an S3 bucket, path-style (`<address>/<bucket>/<prefix><key>`). |
| `--s3-bucket <name>` | `ESCOUADE_TEAM_S3_BUCKET` | none | The bucket, required with S3. |
| `--s3-region <name>` | `ESCOUADE_TEAM_S3_REGION` | `us-east-1` | The region. |
| `--s3-prefix <text>` | `ESCOUADE_TEAM_S3_PREFIX` | empty | Put before every key. |
| none | `ESCOUADE_TEAM_S3_ACCESS_KEY`, `ESCOUADE_TEAM_S3_SECRET_KEY` | none | The S3 keys, through the environment only, never as an argument. |
| `--listen <address:port>` | `ESCOUADE_TEAM_LISTEN` | `127.0.0.1:8443` | Where `serve` listens. |
| `--tls-cert <pem>`, `--tls-key <pem>` | `ESCOUADE_TEAM_TLS_CERT`, `ESCOUADE_TEAM_TLS_KEY` | none | TLS by the server itself; the two go together. Without them, put a reverse proxy in front. |
| `--trust-proxy` | `ESCOUADE_TEAM_TRUST_PROXY=1` | off | The client’s address is the last one of `X-Forwarded-For`, the one the proxy saw. |
| `--metrics-listen <address:port>` | `ESCOUADE_TEAM_METRICS_LISTEN` | off | Prometheus metrics on `/metrics`, at their own address. |
| `--max-file <bytes>` | `ESCOUADE_TEAM_MAX_FILE` | `26214400` (25 MiB) | The largest encrypted file. |
| `--team-quota <bytes>` | `ESCOUADE_TEAM_QUOTA` | `2147483648` (2 GiB) | What a team keeps in all: its streams and its files. |
| none | `RUST_LOG` | `info` | The log’s level. |

`ESCOUADE_TEAM_TRUST_PROXY` takes `1` or `true`. `recovery-token` also reads the team’s id from `ESCOUADE_TEAM_ID`.

## With Docker and Caddy

The repository gives an image (`crates/escouade-team-server/Dockerfile`, built from the repository’s root) and an example compose file in `crates/escouade-team-server/deploy/`: the server, and Caddy in front of it, which gets and renews the certificate. The server is published only on the compose network: only Caddy reaches it, WebSocket included.

```sh
cd crates/escouade-team-server/deploy
export ESCOUADE_TEAM_DOMAIN=team.example.org   # the name members reach, pointed at this machine
docker compose build
docker compose run --rm team-server init       # prints the token that creates the team, once
docker compose up -d
```

- The data is in the `team-data` volume (the SQLite database and the files).
- The metrics listen on port 9464 of the compose network, without being published.
- For PostgreSQL or S3, the compose file has the lines commented out: the `ESCOUADE_TEAM_DATABASE_URL` variable and the `postgres` service, the `ESCOUADE_TEAM_S3_*` variables.

## With systemd

`crates/escouade-team-server/deploy/escouade-team-server.service` runs the server under its own user, apart from any Escouade server mode, in a systemd sandbox (`ProtectSystem=strict`, no capability, `MemoryDenyWriteExecute`, a system call filter). It listens on `127.0.0.1:8443`, behind a reverse proxy on the same machine, and its metrics on `127.0.0.1:9464`.

```sh
sudo install -m 0755 escouade-team-server /usr/local/bin/
sudo useradd --system --home-dir /var/lib/escouade-team-server --shell /usr/sbin/nologin escouade-team
sudo install -m 0644 escouade-team-server.service /etc/systemd/system/
sudo install -d -o escouade-team -m 0700 /var/lib/escouade-team-server
sudo -u escouade-team ESCOUADE_TEAM_DATA=/var/lib/escouade-team-server escouade-team-server init
sudo systemctl daemon-reload && sudo systemctl enable --now escouade-team-server
```

The S3 keys go in a file only root reads, named by `EnvironmentFile=` (the line is commented out in the service file).

## Create the team

1. Run `init`: it prints a token, once. Keep it secret until it is used.
2. The first administrator opens Escouade, Settings › Advanced › “Team server (preview)” › “Open the teams…”, then “Create a team”.
3. They enter the server’s address, the token (“Creation token”) and a name. Escouade creates the team, then shows them its 24-word recovery key and the team’s six-word fingerprint (see [Join a team](team/join)).

The token serves once. For another team on the same server, run `init` again.

## The database

- **SQLite, by default**: `team.db` in `--data`, in WAL (the files `team.db-wal` and `team.db-shm` are there while the server runs).
- **PostgreSQL**: `--database postgres://user:password@host/database`. The connection is made without TLS: keep the database next to the server (the compose network, a Unix socket, a VPN). The service file gives the example of a Unix socket: `postgres://team@/team?host=/run/postgresql`.

On startup, the server creates what its database lacks.

## The files

The encrypted documents and images go, by default, on the disk (`<data>/files`, or `--files`). With `--s3-endpoint` and `--s3-bucket`, they go to a path-style S3 bucket: MinIO, Garage, AWS. Each file is stored under `<team>/<file>`, in hexadecimal, with no name or type.

Today, the app stores no file there yet: they will come with the team’s spec, decisions and documents (coming).

## TLS and the reverse proxy

Members reach the server over HTTPS. Two ways:

- **A reverse proxy** (Caddy, nginx) holds TLS and passes the requests to the server, which listens on `127.0.0.1` or on the compose network. Set `--trust-proxy` so that the rate limit counts per client address, not per proxy address. The proxy must:
  - pass the WebSocket of `/v1/live` (the live broadcast);
  - add the client’s address to `X-Forwarded-For`;
  - let bodies through up to the largest file (25 MiB by default).

  The example `Caddyfile`, in a few lines, does all of this. The repository gives no configuration for nginx.
- **The server itself**, through rustls: `--tls-cert` and `--tls-key`, PEM files the service’s user can read. The certificate is read on startup: after a renewal, restart the server.

Only turn on `--trust-proxy` behind a proxy that is the only one to reach the server: otherwise a client would choose the address the rate limit counts.

## The defenses in place

- Every route asks for a session, except `/healthz`, enrollment by a token, the two steps of the signed challenge, the page of an invitation link and the new device’s side of pairing by a short code.
- A rate limit per address (30 requests a second, 120 at once) and per device (20 a second, 80 at once). Beyond that: `429` and `Retry-After`.
- Bounded sizes everywhere: 1 MiB per stream entry, 25 MiB per file, 2 GiB per team, 500 devices and 10,000 projects per team, 4 live connections per device, 64 KiB of headers read within 15 seconds.

## Back up and restore

Back up the database and the files together:

- with SQLite and the disk: the whole `--data` folder (the compose file’s `team-data` volume, `/var/lib/escouade-team-server` with systemd);
- with PostgreSQL: the database, with its own tools;
- with S3: the bucket.

For a consistent copy of SQLite, stop the server, copy the folder (with `team.db-wal` if it is there), then start it again. To restore, stop the server, put the folder back, start it again.

A backup holds only encrypted bytes and the metadata listed in [the security officer’s guide](team/security#what-the-server-sees): it is not very sensitive, and it is useless without the members’ keys, which never leave their devices.

> [!WARNING]
> Each device remembers what it already read of the team. A backup older than what the members read shows them a rollback: their app says “The server went back on what this device had already read.” and stops reading the team. So restore the most recent backup, and what members wrote since is lost for the server.

## Recover a team

If every administrator of a team loses their devices, its owner keeps the 24-word recovery key. You, the host, give them a token:

```sh
escouade-team-server recovery-token --team <id> --data /var/lib/escouade-team-server
```

The team’s id (32 hexadecimal digits) is shown in Escouade, when the team is created and on its card (“Team id”). The token enrolls one device as an administrator of the server; it gives no key: the 24 words give them. The holder types them in “Recover a team”, with the server’s address and the token.

With Docker: `docker compose run --rm team-server recovery-token --team <id>`.

## Update

1. Back up the database and the files.
2. Build the new version (`cargo build --release -p escouade-team-server`, or `docker compose build`).
3. Replace the binary and restart the service (`sudo systemctl restart escouade-team-server`, or `docker compose up -d`).

Sessions and live connections are held in memory: on restart, the members’ apps reconnect and sign a new session on their own.

**The version handshake.** Each request states the protocol version (`Escouade-Protocol`, `1` today). The server accepts its version and the one before:

- an older app is refused, and tells its user to update Escouade, as the team server no longer speaks that version;
- an app newer than the server is refused, and says the team server is behind and its administrator must update it.

So update the server first: it keeps apps one version behind.

## Monitor

- **`/healthz`** answers `200` and `ok` when the database answers, `503` and `base unavailable` otherwise. It does without the version handshake: a probe reaches it with no header.
- **Prometheus metrics**, on `/metrics` at the `--metrics-listen` address only, never at the exposed address: requests by method, route template and status, refused sign-ins and sessions, rate-limit refusals, entries and files written and their bytes, commits refused because their epoch was taken, frames sent live, the numbers of teams, devices and revoked devices, live connections. No team, project or device id.
- **The log** goes to standard error (`journalctl -u escouade-team-server`, `docker compose logs team-server`), at the `RUST_LOG` level. One line per request: the method, the route template (`/v1/projects/{project}/stream`), the status and the duration. Never an id, an IP address, a token or content. Your reverse proxy’s log, on the other hand, keeps the clients’ addresses: set how long it keeps them.

## What is not there yet

- Signed binaries and a published image (coming).
- The team creation link printed by `init`: today, the token is pasted into Escouade (coming).
- The owner and reader roles on the server, which knows only administrator and member (coming).
- Per-project rights on the server: it serves a project to every device of the team, and only encryption protects a project kept to a few members (coming).
- Several servers on one database: sessions, challenges, rate limits and the broadcast are in memory, one process serves.
