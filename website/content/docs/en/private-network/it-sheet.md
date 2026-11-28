# The sheet for the IT department

For the CISO or the IT department asked to install escouade-connector: what it opens, what it lets in, what it logs, what it keeps, how to stop it, what it does not do and what remains the company’s responsibility.

## In short

A developer of the company has coding agents (Claude Code, Codex…) work on an Escouade server they host outside the company’s network. These agents need a few internal services: a git, a package registry, a staging API. `escouade-connector` is a small program that **the IT department** installs on a machine of its network: it opens an outbound connection to that server, and lets through only the connections to the `host:port` that **the IT department** writes in its file. It is open source (GPL-3.0 or later), written in Rust, without telemetry. Its installation is described in [the connector](private-network/connector#install-the-connector).

## What it opens

- **One outbound TCP connection to a name, on a port**: the Escouade server’s connectors entry, as a WebSocket over TLS (`wss://escouade.exemple.fr/connector`, the address the developer gives). Port 443 is advised.
- **Through the company’s proxy, if needed**: `proxy` in `connector.toml`, or `HTTPS_PROXY`, as `http://` and with Basic authentication only. A TLS inspection is accepted: its authority goes in `authorities`. What the proxy sees go by is the WebSocket request, then encrypted Noise, which it can neither read nor imitate.
- **No inbound port** in the company’s network, and **no port listening** on the connector’s machine: there is no listening socket in its code.
- **The hosts of its ceiling**, from its machine, when the server asks for them.
- **Escouade’s releases** (`github.com`), only if someone runs `update` or sets `auto_update = true`.

## What it lets in

- **TCP connections to the `host:port` of its ceiling alone** (`allow` in `connector.toml`), which the IT department writes itself: `gitlab.acme.internal:22`, `*.npm.acme.internal:443`, an address written as such. The server **cannot widen it**: it receives the ceiling to show it, nothing more. Address ranges (`ranges`) can also bound each address joined.
- **Names are resolved inside the company’s network**, by the DNS of the connector’s machine, once; each address resolved is checked, then joined as such. Never the loopback, the link-local range, a cloud’s metadata, the connector’s machine itself, nor an address outside the ranges: a DNS that changes its answer makes it join nothing else.
- **Bounded**: 256 connections at a time, 32 openings per second, a silent connection closed after 10 minutes, adjustable in `[limits]`.
- **For one server only**: the one whose key was pinned at pairing, by a one-time code valid for 10 minutes. Another server, or a proxy that would pass for it, is refused.

## What it logs

- **Each connection, allowed or refused**, one JSON line on the standard output: journald on Linux (`journalctl -u escouade-connector`), the Event Viewer on Windows (Windows Logs › Application, source `escouade-connector`), the container’s logs.
- **What a line holds**: the time (UTC), the host, the port, the address resolved and joined, the bytes each way, the duration, a refusal’s reason, and the agent and the project announced by the server, as an indication only: the server is the one saying them.
- **The service’s life**: start, connection to the server (its version, the ceiling sent), cut, `connector.toml` read again, update.
- **Never the content** of what goes through, nor the pairing code, nor a private key.

The server keeps its own log of its agents’ connections, which the developer sees in the space.

## What it keeps

- **Its private key** (`connector.key`) and **the server’s public key** with its address (`paired.toml`), in its state folder: `/var/lib/escouade-connector`, in mode 0700, on Linux; `%ProgramData%\escouade-connector` on Windows, the key sealed by DPAPI, the folder open to SYSTEM, the administrators and LocalService alone; a volume in a container.
- **Nothing else**, except, with the automatic update, the downloaded file for the time it takes to put it in place. No cache, no data about the connections.

## How to stop it

- **At once**: stop the service (`systemctl stop escouade-connector`, `sc stop escouade-connector`, `docker compose stop`). Nothing of the company’s network can be reached by the server anymore; the open connections are closed.
- **For good**: `escouade-connector unpair`, which writes zeros over the private key then deletes it; or uninstall (`install-connector.sh --uninstall --purge`, `escouade-connector uninstall --purge`). The developer can also remove it on their side; its key is then worth nothing there.
- **Narrow it without stopping**: remove a line from `allow`. The file is read again within 2 seconds, and an open connection the new ceiling no longer allows is closed.

## What it does not do

- Neither UDP, nor ICMP, nor multicast: TCP only.
- No relay to the Internet: it reaches only its server, the company’s proxy, the hosts of its ceiling and, on request, Escouade’s releases.
- No access to the company’s network for anyone but the paired server.
- Nothing from the company’s network to the server: it carries only the connections the server asks for, to its ceiling.
- No telemetry. Escouade, the publisher, sees nothing: no relay of ours, the tunnel runs from the connector’s machine to the developer’s server.
- No update on its own, unless `auto_update = true`; signed in every case, never to an older version.

## What remains the company’s responsibility

- **Data going out.** An agent that reads the GitLab can send what it reads to the Internet from the server, which has the Internet to reach its models. The connector bounds the way into the company’s network, not the server’s way out. Put in the ceiling only what the company would accept to see read by a contractor.
- **Fine-grained rights.** The ceiling knows neither paths nor methods: credentials to measure, a read-only deploy key, a read-only registry token, a staging account.
- **The connector’s machine**: its updates, its network segment, the proxy’s rule for its way out. The internal hosts see its address as the source.
- **Reading the journal**, and alerting on what matters (a repeated refusal, an unusual volume).
- **Updating the connector**, if it is not automatic.

## The files and their signatures

Each release publishes the Linux archive (x64, arm64, a static program), the Windows `.exe` signed by Azure Artifact Signing, the image `ghcr.io/guillaume-gagnaire/escouade/escouade-connector` (multi-architecture), and for each its SHA-256 and its signature by Escouade’s update key, with a provenance attestation (`gh attestation verify`). The code is public: `crates/escouade-connector` in Escouade’s repository.
