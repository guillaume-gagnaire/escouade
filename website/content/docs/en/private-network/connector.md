# The connector

The escouade-connector program, which the company runs on its side: it opens an outbound connection to your server and lets through only the hosts of its list. Install it, pair it, write its list, choose a project’s, check it, update it, remove it, and its limits.

> [!WARNING]
> The connector is a preview of 2.3, like [server mode](server): its files, its image and its install script are published with the 2.3 release.

## How it works

```
Escouade server (your VPS)                          Acme’s network
  agents ──▶ the gate (127.0.0.1) ──▶ the entry ◀══ TLS, outbound ══ escouade-connector
                                                                    │ its ceiling: gitlab:22, nexus:443
                                                                    ▼
                                                             gitlab, nexus, api-recette
```

- **The connector opens the connection**, from the company to your server: a WebSocket over TLS, through the company’s proxy if there is one. No inbound port at the company, no port listening on its machine. Inside, a Noise handshake whose two keys are pinned at pairing: neither a proxy that inspects TLS nor another server can pass for either side.
- **The server asks it**, connection by connection, to “open `gitlab.acme.internal:22` for such agent of such project”. The connector checks its ceiling, resolves the name inside the company’s network, checks the address, opens it and passes the bytes.
- **Two lists**: the **ceiling**, in `connector.toml`, written by the IT department on its machine, which the server cannot widen; and **each project’s list**, on the server, chosen by you within that ceiling, empty by default.
- **The agent uses it without knowing.** On the server, the gate becomes the agents’ one proxy (`HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY` to `127.0.0.1`), and their `ssh` goes through it (`ProxyCommand="escouade-server via %h %p"`). A host of the project’s list goes to the connector; the rest of the Internet (the models, public npm, GitHub) goes out as before, directly or through the space’s proxy.

## Open the server’s entry

The connectors’ entry is off by default. It is apart from the remote API: a port of its own, which speaks only the connectors’ protocol.

```sh
escouade-server --connector-listen 0.0.0.0:8443 --connector-url wss://escouade.exemple.fr:8443/connector
```

| Option | Variable | What it does |
|---|---|---|
| `--connector-listen <address:port>` | `ESCOUADE_SERVER_CONNECTOR_LISTEN` | Opens the entry, and the agents’ gate with it. An address and a port, always: `0.0.0.0:8443` |
| `--connector-cert <file>`, `--connector-key <file>` | `ESCOUADE_SERVER_CONNECTOR_CERT`, `ESCOUADE_SERVER_CONNECTOR_KEY` | Its TLS certificate, in PEM. Without them, the entry is behind a domain’s TLS proxy (Coolify’s Traefik, Caddy) |
| `--connector-url wss://…/connector` | `ESCOUADE_SERVER_CONNECTOR_URL` | Its address as the connectors reach it, which pairing gives them. Without it, made from the listening address, and said to be “guessed” in the log |
| `--gate` | `ESCOUADE_SERVER_GATE` | The gate alone, without an entry |

- **Port 443 goes through everywhere.** A company proxy lets 443 out; another port asks one more rule of the IT department. The simplest: a domain behind your host’s TLS proxy, which leads to the entry’s port, and `--connector-url wss://escouade.exemple.fr/connector`. Declare that proxy (`--trust-proxy`) so that each source counts its attempts apart.
- **In Docker**, publish the entry’s port (`-p 8443:8443`) or lead it through your host’s proxy; the remote API stays on `127.0.0.1`.
- **A policy** can forbid it: `servers.privateNetwork` set to `false` (see [the computer’s policies](managed-policies)) prevents the entry, the gate and pairing.

## Add a connector

On the server’s space, Space settings › Private network › “Add a connector”: give it the network’s name (“Acme”), then “Make the code”. Or, on the server:

```sh
escouade-server connector pair Acme
```

You get an 8-letter code, like `ABCD-EFGH`, valid for 10 minutes and once, the entry’s address and the command to give the IT department: `escouade-connector pair wss://escouade.exemple.fr:8443/connector ABCD-EFGH`. The code is shown once; Escouade does not keep it.

- The code never travels over the network, even under a TLS inspection: it is the key of the pairing handshake.
- A wrong, expired or already used code is refused without a word, and never cancels the code that waits. Attempts are bounded: 20 failures per minute and per source, 5 wrong codes per source during a code’s life, 20 failures per minute for the whole entry; a connector already paired always gets in.
- Neither the code nor a private key is written to the log.

## Install the connector

On one of the company’s machines that reaches the hosts your projects need. The IT department does it, with [the sheet](private-network/it-sheet) in hand.

### Linux

Linux x64 or arm64 with systemd. The script checks the archive’s SHA-256 sum and its signature, puts the program in `/usr/local/bin` and a locked-down systemd service, and starts nothing before pairing.

```sh
curl -fsSLo install-connector.sh https://github.com/guillaume-gagnaire/escouade/releases/latest/download/install-connector.sh
less install-connector.sh
sudo sh install-connector.sh
sudoedit /etc/escouade-connector/connector.toml
sudo escouade-connector pair wss://escouade.exemple.fr:8443/connector ABCD-EFGH
sudo escouade-connector check
sudo systemctl enable --now escouade-connector
```

The service runs under an account of its own without a shell or a home folder, the system read-only, without any capability, IPv4 and IPv6 alone. Its journal goes to journald: `journalctl -u escouade-connector`. A proxy’s password rather goes in `/etc/escouade-connector/proxy.env` (`HTTPS_PROXY=…`, mode 0600, read by systemd alone), as `connector.toml` is readable by the machine’s accounts.

### Windows

Windows x64: `escouade-connector-<version>-windows-x64.exe`, signed. In an administrator console:

```powershell
escouade-connector install
notepad C:\ProgramData\escouade-connector\connector.toml
escouade-connector pair wss://escouade.exemple.fr:8443/connector ABCD-EFGH
escouade-connector check
sc start escouade-connector
```

`install` copies the program to `%ProgramFiles%\escouade-connector` and makes the `escouade-connector` service under `LocalService`, started with the machine and restarted after a failure. Its journal goes to the Event Viewer (Windows Logs › Application, source `escouade-connector`). Its key is sealed by DPAPI for the machine, its folder open to SYSTEM, the administrators and LocalService alone.

### The image

`ghcr.io/guillaume-gagnaire/escouade/escouade-connector`, for `linux/amd64` and `linux/arm64`: the program alone, without a shell, an unprivileged user, read-only, without capabilities. The repository gives an example Compose file and Kubernetes manifest (`deploy/connector/`):

```sh
docker compose run --rm connector pair wss://escouade.exemple.fr:8443/connector ABCD-EFGH
docker compose up -d
docker compose logs -f connector
```

`connector.toml` is mounted read-only beside the Compose file, the keys kept in a volume. No port is published. One replica: one key pair, one connector.

## Write `connector.toml`

The IT department writes it. On Linux `/etc/escouade-connector/connector.toml`, on Windows `%ProgramData%\escouade-connector\connector.toml`, elsewhere `--config <file>`.

```toml
server = "wss://escouade.exemple.fr:8443/connector"     # optional: pair gives it too
proxy = "http://proxy.acme.internal:3128"               # optional: else HTTPS_PROXY and NO_PROXY
authorities = "/etc/escouade-connector/acme-root.pem"    # more authorities (a TLS inspection)
allow = [
  "gitlab.acme.internal:22",
  "*.npm.acme.internal:443",
  "10.20.1.5:8443",
]
ranges = ["10.20.0.0/16"]                                # optional: every address joined is in them
auto_update = false

[limits]
max_streams = 256        # from 1 to 4096
opens_per_second = 32    # from 1 to 1024
idle_minutes = 10        # from 1 to 1440
```

- **The ceiling**, `allow`: `host:port`; `*.suffix:port` for every name under the suffix, not the suffix itself; an address written as such (`10.20.1.5:8443`, `[fd00::5]:22`), the only way to allow an address. Empty, nothing goes through.
- **`ranges`**: optional; each address joined must be in them.
- **The company’s proxy**, for the connector’s way out: `http://` only, with Basic authentication.
- **An unknown key is an error**, as is an unreadable rule, said with its line: a typo does not pass for a forgotten rule.
- **Read again live**, every 2 seconds: the change is said in the journal and sent again to the server; a stream the new ceiling no longer allows is closed. A file that no longer reads is said, and the last one read is kept.

### What the connector always refuses

For each connection, in order: its bounds (`max_streams`, `opens_per_second`); the ceiling, once the name is normalized; one resolution only, by its machine’s DNS; then each address resolved is checked. Never the loopback, the link-local range, a cloud’s metadata (`169.254.169.254`), the unspecified address, broadcast, an address of its own machine nor an address outside `ranges`, even hidden in an IPv6 one. The checked address is joined as such, without a second resolution: a DNS that changes its answer makes it join nothing else.

## Pair and check

- **`escouade-connector pair <address> <code>`** (or `-` instead of the code, to read it on the standard input, out of the process list): it makes a new key pair, pairs, and keeps them only once the server agrees. It says the address and both public keys, never the code. A wrong code: “the code is wrong, expired or already used (ask a new one)”.
- **`escouade-connector check`**: for each rule of the ceiling, the resolution, the checks and a connection, as for an agent: “ok gitlab.acme.internal:22: … answers at 10.20.1.5:22 (12 ms)”, or the reason. It also says whether it is paired. Exit code 1 if a host does not answer.
- **In the space**, Space settings › Private network shows each connector: connected since, cut since, its version, its ceiling as it sent it.

The connector’s texts (commands, journal, reasons) are in English.

## Choose a project’s list

A project reaches nothing of a private network until you choose it: Project settings › Private network, the hosts of each connector’s ceiling this project needs. A pattern of the ceiling (`*.npm.acme.internal:443`) is narrowed to a name (`registry.npm.acme.internal`). A host outside the ceiling is refused: only the IT department widens it, in `connector.toml`.

The repository’s `.escouade.yml` can propose it:

```yaml
private_network:
  hosts:
    - gitlab.acme.internal:22
    - nexus.acme.internal:443
```

Nothing is reached before your agreement (“Accept this list”); if the list changes, the agreement is asked again. A team’s `.escouade.yml` can recommend it or impose it (see [.escouade.yml](escouade-yml)). **No agent** reads or writes it: a booby-trapped repository or a hijacked agent do not open a company’s network on their own.

## What the agent sees

It runs `git clone git@gitlab.acme.internal:acme/api.git`, `npm install` or its tests as usual. A refusal is told to it plainly, and it can report it to you:

- “gitlab.acme.internal:22 is not in the list of the project Demo”: add the host in Project settings › Private network;
- “… is not allowed by the connector of Acme”: it is outside the ceiling, to ask the IT department;
- “the private network of Acme is unreachable since 14:32”: the connector is cut.

A cut connector: the gate refuses at once instead of letting the agent wait; the open connections are closed, without resuming (a `git fetch`, an `npm install` are run again); the connector reconnects on its own. Cut for more than 10 minutes while a project needs it, it leaves a request in [the “Your turn” box](inbox).

### Java and what ignores proxies

Maven, Gradle, Node’s `fetch` without `NODE_USE_ENV_PROXY` or a program that opens its sockets itself do not follow `HTTPS_PROXY`. In the server’s image, `--gate-loopback` (`ESCOUADE_SERVER_GATE_LOOPBACK=1`) gives each host of the lists a loopback address where the gate listens; `escouade-server gate-hosts` writes the `extra_hosts` lines to put in the Compose file. TLS stays end to end: the company’s internal authority must be in Java’s store.

## The journal

- **At the connector**: one JSON line per event, on the standard output (journald, the container’s logs) or in the Event Viewer. Each connection allowed (`allowed`, then `closed` with the bytes and the duration) or refused (`refused`, with its code and its reason): the time, the host, the port, the address joined, the agent and the project announced by the server. Never the content, the pairing code nor a private key.
- **At the server**: Space settings › Private network › “Journal”, the agents’ last 1,000 connections through the gate, allowed or refused, with the route and a refusal’s reason. It starts again from zero when the server restarts.

## Update

The server accepts a connector of its version and of the two minor ones before; at two minor versions, the space says “to update”. Beyond, or newer than the server, the channel is refused, and the state says what to do.

- **By hand**, by default: `sudo escouade-connector update` (as an administrator on Windows), `--to X.Y.Z` for a given version, `--check` to only say it. The signature is checked before anything is touched, never an older version, the old program kept as `escouade-connector.old`, the service restarted.
- **Automatic**, if the IT department asks for it: `auto_update = true` in `connector.toml`. It looks once a day, and updates only when no connection is open.
- **In the image**, a new image: `docker compose pull && docker compose up -d`.

## Remove a connector

- **On your side**: Space settings › Private network › “Remove”, or `escouade-server connectors remove Acme`. Its key is worth nothing anymore, its channel is closed within two seconds, and its trace is kept (name, date, by whom). `escouade-server connectors` lists them.
- **On the company’s side**: stop the service, or `escouade-connector unpair`, which writes zeros over the private key then deletes it. To remove the program: `sudo sh install-connector.sh --uninstall` (`--purge` also removes `connector.toml` and the key), `escouade-connector uninstall` on Windows.

To bring it back, it takes a new code and a new pairing.

## The limits

- **TCP only**: neither UDP (an internal DNS queried directly, Kerberos over UDP, QUIC), nor ICMP (`ping`, `traceroute`).
- **Nothing from the company’s network to the server**: a webhook from an internal CI does not reach Escouade through the connector.
- **The source address** the company’s hosts see is the connector machine’s: a rule that allows only some addresses must allow it.
- **The company’s proxy**, for the connector’s way out: `http://` and Basic only, neither NTLM nor Kerberos. A server machine often has a rule of its own in the proxy.
- **Java**: Maven and Gradle do not follow `HTTPS_PROXY`; their proxy config, or the image’s loopback names, and the internal authority in their certificate store.
- **A rule, not a wall**: a project’s list is an instruction given to its agents. All the agents of a server share one account, and a hijacked agent can ignore its variables or pass for another project. What really holds is the connector’s ceiling.
- **The way out to the Internet is not bounded**: the connector bounds the way into the company’s network, not your server’s way out to the Internet, which the agents need for their models. An agent that reads an internal host can send what it reads elsewhere.
- **What the list does not say**: neither paths nor methods. A `push` to another branch of the GitLab goes through; fine-grained rights remain the accounts’ (a read-only deploy key, a read-only registry token).
