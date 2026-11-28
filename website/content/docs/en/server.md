# Server mode

Run Escouade around the clock on a server with no desktop, a VPS or a container, and drive it from your computer: the Docker image, Compose, Coolify, the systemd service, pairing, access, backups, updates and what the server keeps.

> [!WARNING]
> Server mode is a preview of 2.3. The `escouade-server` program, its image and its install script are published with the 2.3 release; the client’s “Connect to a server” screen comes with it.

## What the server does

`escouade-server` is Escouade’s core without a window: the projects, the agents and their harnesses, the worktrees, the board and the autopilot, the terminals, the MCP server. It runs on the server, and Escouade on your computer is only a view of it: closing the client, shutting down the computer or losing the network touches no agent.

It serves Escouade’s remote API on `127.0.0.1`, port `46480`, and nothing else: it never exposes itself. You reach it through an SSH tunnel (see [Connect from your computer](#connect-from-your-computer)). It exists for Linux x64 and arm64.

## With Docker

The image `ghcr.io/guillaume-gagnaire/escouade/escouade-server` (linux/amd64 and linux/arm64) is ready to use. In three commands, on the server:

```sh
docker volume create escouade-data
docker run -d --name escouade --restart unless-stopped --stop-timeout 60 \
  -e ESCOUADE_SERVER_LISTEN=0.0.0.0 -p 127.0.0.1:46480:46480 \
  -v escouade-data:/data --read-only --tmpfs /tmp --cap-drop ALL --security-opt no-new-privileges \
  ghcr.io/guillaume-gagnaire/escouade/escouade-server:2.3
docker exec escouade escouade-server pair
```

The last one prints the pairing code (see [Pairing](#pairing)).

- **What it holds.** A slim Debian 13, `escouade-server` and the seed of its plugins, `git`, `openssh-client`, `curl`, `tini` (which reaps the harnesses’ orphaned processes), Node.js and npm (for the harnesses installed with `npm install -g`). It weighs 427 MB uncompressed, about 150 MB to download. No harness and no account: you install and sign them in from Escouade.
- **A user without privileges.** It runs as `escouade` (uid 10001), with no root and no sudo, a read-only filesystem (`--read-only`) and no capability at all (`--cap-drop ALL`).
- **Everything in `/data`.** `HOME` is `/data`: Escouade’s data (`/data/.escouade`), your projects and their worktrees (clone them into `/data/projects`), the harnesses and their sign-ins (`/data/.local`, `/data/.npm-global`, `/data/.claude`…). Nothing is written anywhere else, except `/tmp`, in memory.
- **Nothing exposed by default.** Without `ESCOUADE_SERVER_LISTEN`, the API listens on the container’s `127.0.0.1` alone: even a published port does not reach it. `ESCOUADE_SERVER_LISTEN=0.0.0.0` opens it to the container’s network, for the port published on the host’s `127.0.0.1` only. The API is then **in clear** there (HTTP and WebSocket without TLS): any other container of that network (on Coolify, the other resources of the same network) reaches it and may try pairing codes. TLS is the proxy’s: put one in front (Coolify’s Traefik, Caddy) or go through an SSH tunnel, never a port published on a public address.
- **Its health.** Its `HEALTHCHECK` runs `escouade-server health`, which checks that the server runs and writes its status. `docker ps` says “healthy”.
- **Its stop.** `docker stop` sends SIGTERM: the turns under way are cut cleanly, the state is written, and each agent resumes its session at the next start. Give it time (`--stop-timeout 60`).
- **At its first start**, on an empty volume, the log (`docker logs escouade`) says what to do: run `docker exec <container> escouade-server pair`. It never prints the code itself.

A volume mounted from a folder of the host (`-v /srv/escouade:/data`) must belong to uid 10001: `sudo chown 10001:10001 /srv/escouade`.

## With Docker Compose

The repository gives an example, `deploy/docker-compose.yml`: the port published on the host’s `127.0.0.1` only, a read-only filesystem, no capability, the advised limits.

```sh
curl -fsSLO https://raw.githubusercontent.com/guillaume-gagnaire/escouade/main/deploy/docker-compose.yml
docker compose up -d
docker compose exec escouade escouade-server pair
```

## On Coolify

`deploy/coolify/docker-compose.yml` is the Compose file as Coolify takes it: a “Docker Compose” resource, this file pasted as it is. Ask for the pairing code from the resource’s terminal: `escouade-server pair`.

As it is, nothing goes on the Internet: the API is published on the Coolify server’s `127.0.0.1`, and you reach it through an SSH tunnel to it. The same file offers, commented out, a domain behind Coolify’s proxy (Traefik, in TLS): two lines to uncomment, `SERVICE_FQDN_ESCOUADE_46480` and `ESCOUADE_SERVER_ALLOW_HOSTS`, and the `ports` to remove. Read [its risks](#a-tls-proxy-in-front) first.

## Without Docker, with systemd

`install.sh` puts the program and a systemd service in place, on Linux x64 or arm64 with systemd (Debian 12, Ubuntu 22.04 and newer).

```sh
curl -fsSLO https://github.com/guillaume-gagnaire/escouade/releases/latest/download/install.sh
less install.sh
sh install.sh
```

- **What it checks.** It downloads the release’s archive for your machine, checks its SHA-256 and its signature, made by the same key as the app’s updates (with `minisign` if it is there, else `openssl`). An archive whose signature does not hold is refused, and nothing is changed.
- **A service of your user** by default (`systemctl --user`): the program in `~/.local/share/escouade-server`, a link in `~/.local/bin`, the service in `~/.config/systemd/user`. It turns on `loginctl enable-linger`, so that the service also runs with no session open.
- **A service of the system** with `sudo sh install.sh --system`: it creates the user `escouade` (home `/var/lib/escouade`) if it does not exist, and the service runs as it. `--user <name>` picks another user.
- **Run again**, it changes only what differs (another version, another service); at the same version, the server is not restarted. `--version X.Y.Z` targets a given version.
- **To uninstall**: `sh install.sh --uninstall` removes the service and the program, and keeps the data. `--uninstall --purge` also removes `~/.escouade`; with `--system`, the user `escouade` and its whole home (the harnesses and their sign-ins included).

At the end, it says how to pair your computer. With `--system`, commands run as the service’s user: `sudo -u escouade -H escouade-server pair`.

## Pairing

1. On the server: `escouade-server pair` (in Docker: `docker exec escouade escouade-server pair`). It prints a code, like `8NUC-W6HS`, good once and for 10 minutes; 5 wrong tries end it.
2. In Escouade, on your computer: “Connect to a server”, the SSH host, then this code and a name for the device.
3. The computer receives its own token, which it keeps. `escouade-server devices` lists the paired devices, `escouade-server devices revoke <id>` revokes one: its connections close within seconds.

Neither the code nor the token is ever written to the log.

## Connect from your computer

### The SSH tunnel

This is the advised access. A server is already administered over SSH, with your keys:

```sh
ssh -N -L 46480:127.0.0.1:46480 <your server>
```

Escouade on your computer then targets `127.0.0.1:46480`, on your side, which the tunnel carries to the server. Nothing new is opened on the server: in Docker, the port is published on the host’s `127.0.0.1` only (`-p 127.0.0.1:46480:46480`), never on its public address.

Its risks: anyone with an SSH account on the server, or a process of the host, reaches the local port and may try to pair (without a valid code, they are refused). The access is as safe as your SSH: keys, no password.

### A TLS proxy in front

The server can also be reached by a domain, behind a proxy that holds TLS (Coolify’s Traefik, Caddy, nginx). The server listens on the container’s network (`ESCOUADE_SERVER_LISTEN=0.0.0.0`) and takes the domain’s name in the `Host` header (`ESCOUADE_SERVER_ALLOW_HOSTS=escouade.example.org`, or `--allow-host`); any other name is still refused. A request that gives `Host` or `Origin` twice, or whose full address names another host than its `Host`, is refused: the proxy and the server always read the same name.

Its risks: the API is then on the Internet. Anyone who reaches the domain may try to pair (a 40-bit code, good for 10 minutes, once, 120 tries read a minute at most) or to guess a token (256 bits); every try is in the log. A wrong code never cancels yours, and tries from the Internet never block the one you type on the server or through SSH. Declare the proxy (`--trust-proxy <its address>`, or `ESCOUADE_SERVER_TRUST_PROXIES`): each client behind it then counts its failed tries apart, from the `X-Forwarded-For` header the proxy writes; otherwise they all count as one source. A flaw of the server would be exposed to everyone: keep this mode to a domain filtered by IP address on the proxy, or to a private network (Tailscale, WireGuard). Never without TLS: the API encrypts nothing itself. Escouade’s client connects through the SSH tunnel today; a domain’s address (`wss://`) comes with its screen.

## The harnesses on the server

The agents use the harnesses installed on the server. Escouade installs them with their plugin’s command, as on your computer: for Claude Code, `curl -fsSL https://claude.ai/install.sh | bash`, which puts it in `~/.local/bin` (in Docker: `/data/.local/bin`, in the volume); Codex, Copilot CLI, Gemini CLI and OpenCode install with `npm install -g` (in Docker, into `/data/.npm-global`). They survive a new image.

Signing an account in opens on your computer: the harness’s sign-in address opens in your browser, and the account stays on the server.

## What the server keeps

Everything Escouade keeps on a computer, it keeps on the server: the projects, the conversations, the board, the stats (see [Local data](data)). Two differences:

- **The accounts’ sign-ins live on the server.** The harnesses sign in there (Claude Code, Codex…) and keep their tokens there. The server’s security becomes your accounts’: its disk, its SSH access, its backups.
- **The secrets are in a file.** A server has no keychain: the devices’ tokens, the keys of the integrations, of Cursor, of the online voices and of the plugins are in `~/.escouade/secrets.json`, readable by its owner alone (mode 0600, set back to 0600 at start if it is not).

## Back up

Everything is in the volume (`/data`), or in the home of the service’s user (`~/.escouade` and the harnesses’ folders). Back it up with the server stopped, for a consistent state:

```sh
docker stop escouade
docker run --rm -v escouade-data:/data -v "$PWD":/backup debian:trixie-slim tar -czf /backup/escouade-data.tar.gz -C /data .
docker start escouade
```

A backup holds the accounts’ sign-ins and the secrets: keep it encrypted, as the server is.

## Update

The server updates **itself**. Shortly after it starts, then every 15 minutes, it looks for a version of its channel: stable, or beta when it was installed as a beta (`install.sh --version X.Y.Z-beta.N`) or put on beta. It downloads it, checks its signature as `escouade-server update` does, puts it in place, then restarts **at rest** only: no turn under way, no question waiting, no background task, no launch, no terminal used in the last 5 minutes. systemd starts it again on the new version. It never takes an older version than its own, and an archive whose signature does not hold changes nothing. Its log says each version found, checked, put in place and started again, and each refusal with its reason.

To turn it off or change the channel, on the server:

```sh
escouade-server config                      # what is set
escouade-server config automatic off        # it says the version found, putting nothing in place
escouade-server config channel beta         # or stable
```

From your computer, on the server’s space, **Settings › General** shows its version, the one it found and these two settings; “Update the server” asks it. A policy of your company or your team may impose either (`updates.automatic`, `updates.channel`), as for the app. Without systemd, the server puts nothing in place itself: it says the version and the command to run.

- **In Docker**, an update is a new image: `docker compose pull && docker compose up -d` (or `docker pull`, then the container made again on the same volume). The agents resume their sessions. `escouade-server update` refuses to run in the image, and the server never replaces anything there: it says that a new image exists, in its log and in your Settings, with the `docker pull` command.
- **With systemd, by hand**: `escouade-server update`. It reads the latest version (`--to X.Y.Z` for a given version, `--beta` for the higher of the beta and stable channels, `--check` to only say it), downloads the archive, checks its signature with the key of the app’s updates and that this signature names this very archive, then puts each file in place of the old one, kept as `.old`. It then waits for the server to be at rest (no turn under way, no question waiting, no background task, no terminal used in the last 5 minutes); in a terminal, it offers to restart right away, cutting the turns, and `--now` does so without waiting. systemd starts the server again on the new version. It never goes back to an older version than its own unless it is named: `--to X.Y.Z --downgrade`.

When your computer’s version and the server’s differ, the server refuses the connection and says which to update. When it is the server, the space says both versions and offers **“Update the server”**: once you confirm, your computer asks it with its token (a phone cannot), and the server moves to your version, then restarts at rest; your computer reconnects. In Docker, the screen gives the `docker pull` command to copy on the server.

## Advised limits

Measured on the image, on an x64 computer: the server at rest, its plugins loaded, takes about 235 MB and 106 tasks (processes and threads); an agent of the tests’ fake `claude` in the middle of a turn adds about 18 MB and 7. A real harness weighs more: count several hundred MB per working agent, plus what its commands start (builds, tests). The Compose example sets `mem_limit: 4g` and `pids_limit: 2048`: enough for a few agents at a time; raise them with the number of agents in parallel. On a server, the Kanban takes up to 64 tickets at once (16 on a computer); beyond 6, Settings › Kanban says what the server’s machine holds.

## Commands

| Command | What it does |
|---|---|
| `escouade-server` or `escouade-server run` | The server, until SIGTERM |
| `escouade-server pair` | A pairing code for a new device |
| `escouade-server devices` | The paired devices |
| `escouade-server devices revoke <id>` | Revokes a device |
| `escouade-server health` | Exit code 0 if the server runs and answers, 1 otherwise (the image’s `HEALTHCHECK`) |
| `escouade-server update` | Puts the server at the latest signed version (`--to`, `--beta`, `--now`, `--check`) |
| `escouade-server config` | The automatic update and its channel (`automatic on` or `off`, `channel stable` or `beta`) |

Each option has its environment variable; the option wins over the variable.

| Option | Variable | Default |
|---|---|---|
| `--data-dir <folder>` | `ESCOUADE_SERVER_DATA` | `~/.escouade` |
| `--port <n>` | `ESCOUADE_SERVER_PORT` | `46480` |
| `--listen <address>` | `ESCOUADE_SERVER_LISTEN` | `127.0.0.1` |
| `--allow-host <name>` | `ESCOUADE_SERVER_ALLOW_HOSTS` (names separated by commas) | none |
| `--trust-proxy <address>` | `ESCOUADE_SERVER_TRUST_PROXIES` (addresses separated by commas) | none |
| `--debug` | none | off |

`--listen` on another address than `127.0.0.1` warns in the log: that port must never be open to the Internet.
