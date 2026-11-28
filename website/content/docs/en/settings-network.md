# Settings: the network (Advanced)

The “Network” group of the “Advanced” page configures the HTTP(S) proxy and TLS certificates (an extra authorities file, the checking) for Claude Code, quotas, integrations, updates, and dictation (the download of its models and, online, its service).

## Open the page

Open the settings with `Ctrl+,` (`⌘,` on macOS), then choose “Advanced”: “Network” is its first group; “proxy” in the bar’s search leads there too. These are the app’s settings, shared by all projects, and they apply as you set them.

They matter behind a company network: a proxy to get out, and sometimes a certificate the proxy imposes. Without a proxy, there’s nothing to set here.

## Proxy

The app says so under the group’s title: “Claude Code processes, quota reads, integrations, and updates all go through the proxy. It applies to agents the next time their process (re)starts.”

| Setting | Effect | Default |
|---|---|---|
| “HTTP(S) proxy” | The proxy’s address, for example `http://user:password@proxy:3128`. Empty (“none”), connections are direct. The password goes to the system keychain: see [“The proxy’s password”](#the-proxy-s-password). | Empty |
| “Never go out without the proxy” | When the proxy doesn’t answer, an update or a download fails with its reason instead of going out directly. Grayed with no proxy. | On as soon as a proxy is set |
| “Exclusions” | The hosts that don’t go through the proxy, separated by commas (the `NO_PROXY` list). | `localhost,127.0.0.1` |
| “Also apply the proxy to the built-in terminals” | Terminals also get the proxy variables. | Off |

### What goes through the proxy

- **Claude Code processes**: agents, and the other calls to Claude Code that Escouade makes (naming an agent, commit messages, signing in to an account). They get `HTTPS_PROXY`, `HTTP_PROXY`, their lowercase variants, and `NO_PROXY` (with `no_proxy`) if “Exclusions” isn’t empty. An agent only sees them the next time its process (re)starts: an agent already running keeps its settings until then.
- **Quota reads and integrations** (Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues, Azure Boards): the app’s requests go through the proxy and respect “Exclusions”.
- **Updates**: checking and downloading go through the proxy. An invalid proxy address makes the check fail, with the message “invalid proxy address”. When the proxy itself doesn’t answer (switched off, or a computer outside the company network), the update fails and says so, as long as “Never go out without the proxy” is on; off, Escouade tries once without the proxy and the failure message gives both tries. When the proxy refuses the request, it is never made again without the proxy. See [“Behind a company proxy”](updates#behind-a-company-proxy).
- **Built-in terminals**: only if “Also apply the proxy to the built-in terminals” is on. The same variables are then added to terminals, launch commands, and worktree commands. See [terminals](terminals).

Without a proxy address, Escouade adds no proxy variable: processes keep your session’s environment.

### The proxy’s password

Type the address with your username and password, as before. Escouade keeps the password in the system keychain (Windows Credential Manager, the macOS Keychain, the desktop’s secret service on Linux), never in `settings.json`: from the next start, the field shows the address without it (`http://user@proxy:3128`), and says so below. Agents and the app’s connections still get the whole address. To change the password, type the address with the new one; another username, or an address with no username, takes it out of the keychain.

A password an earlier version wrote in `settings.json` is taken out of it at the first start, and goes to the keychain. Without a keychain (Linux without a secret service, WSL), it stays in `settings.json`, apart from the address, readable by you alone.

### Never go out without the proxy

On as soon as a proxy is set, until you turn it off. When the proxy doesn’t answer, checking for or downloading an update fails with its reason, for example “couldn’t reach github.com through the proxy 127.0.0.1:9000 (…); nothing is tried without the proxy: “Never go out without the proxy” is on (Settings › Advanced › Network)”. Nothing goes out directly, which a company where every connection goes through the proxy expects. Likewise, if the proxy’s password is no longer in the keychain at start, agents don’t start and say so, until you type the address again with its password. Turn it off for Escouade to try once without the proxy when yours can’t be reached (a laptop outside the company network, for example); what it downloads stays checked by its signature.

## Certificates

The app says so under the file’s setting: “Escouade trusts recognized certificates, those installed on this system and those of the authorities file. Agents get the file the next time their process starts.”

| Setting | Effect | Default |
|---|---|---|
| “Extra certificate authorities” | A `.pem` file (the root certificate of a corporate proxy, Zscaler’s for example) that Escouade trusts on top of the system’s store. “Choose…” picks it; under the field, the app says how many certificates it reads in it, or why it doesn’t read it. | Empty |
| “Skip TLS certificate verification” | Only for the connections that go through the proxy above; with no proxy, it does nothing. Kept for the installations that have it: prefer the authorities file. | Off |

### The authorities file

It serves every connection of Escouade: quotas, integrations, updates, the store, dictation and the plugins’ network (their requests and their WebSockets). Agents get it the next time their process starts: `NODE_EXTRA_CA_CERTS` for the Node harnesses (Claude Code, Copilot CLI, Cursor CLI, Gemini CLI, OpenCode) and `CODEX_CA_CERTIFICATE` for Codex. The commands agents run (npm, node…) inherit these variables. Recognized certificates and the system’s still hold: the file adds to them.

### Skipping verification

When on, the setting holds only for the connections that go through the configured proxy: to a host of “Exclusions”, or with no proxy, the certificate is checked. Agent processes get `NODE_TLS_REJECT_UNAUTHORIZED=0` only when a proxy is configured. While the box is ticked, a band at the top of the window says so, and its “Choose the root certificate” button leads to the authorities file.

> [!WARNING]
> An intercepted connection, tokens included, would no longer be detected. If you take your computer to another network with the same proxy, its connections would stay unchecked.

### From the box to the file (Zscaler)

Behind Zscaler or another proxy that inspects TLS:

1. Get the proxy’s root certificate. On Windows, open `certmgr.msc`, “Trusted Root Certification Authorities › Certificates”, find the proxy’s (“Zscaler Root CA”), then right-click, “All Tasks › Export…”, as “Base-64 encoded X.509 (.CER)”. On macOS, export it from “Keychain Access” as `.pem`. Your IT department can also give it to you.
2. In Settings › Advanced › Network, “Extra certificate authorities”, “Choose…” and pick that file: the app should say “1 certificate read”.
3. Untick “Skip TLS certificate verification”: the band goes away.
4. Restart the running agents (or wait for their next start): they take the file then.

If a connection still fails with “UnknownIssuer”, the proxy shows another certificate than the one chosen: take the one the browser shows for the same site.

## Offline mode

“Offline mode”, at the bottom of the “Network” group, stops in one gesture the four flows Escouade starts by itself: what to know before you set a firewall.

| Flow stopped | What it does otherwise |
|---|---|
| The updates | The search for a new version of the app, every 5 minutes, and for the installed plugins’, at the start. |
| The store | The reading of store.escouade.dev’s catalog, at the start then every 6 hours. |
| The revocation list | Read with the catalog, with its copy and its attestation on escouade.dev. |
| The statistics | The anonymous usage statistics (if you accepted them), every minute, and the count of the store’s installs. While the mode lasts, nothing is kept to leave later. |

While it is on, the status bar says “Offline”; a click opens the setting. “Check for updates” and “Check now” say so instead of searching: “Offline mode: offers of dd/mm hh:mm”. The catalog and the revocation list kept on the machine still apply.

What you ask for by a gesture is not stopped: installing a plugin, showing an image of a message. Nor is the agents’ work, `git fetch` or the reading of the quotas.

### Imposed by the device

A managed device may impose it with the policy `network.offline` (true). The setting is then on and greyed out, with “Imposed by the policy of this device.”, whatever the setting says. Escouade reads it:

- on Windows, in `HKLM\SOFTWARE\Policies\Escouade` then `HKCU\SOFTWARE\Policies\Escouade`: a `REG_DWORD` value named `network.offline`, set to 1;
- on macOS, in the managed preferences of the domain `dev.gagnaire.escouade` that a configuration profile installs (`/Library/Managed Preferences/`): the key `network.offline`, a Boolean;
- on Linux, in `/etc/escouade/policies/*.json`: `{"network.offline": true}`, the folder and the files owned and written by root alone.

A policy put while the app runs applies within the minute.

## On a server, with the private network’s gate

On an Escouade server started with the private network’s gate (`--gate`, or `--connector-listen` for the connectors), the agents, the terminals, the launches and Escouade’s `git` no longer get the proxy set here: they get the server’s gate, on `127.0.0.1`, and the proxy set here goes behind it, with its password and its exclusions, which go out directly. The terminals get it even with “Also apply the proxy to the built-in terminals” unchecked. See [the connector](private-network/connector) and [Reach a private network](private-network/introduction). This computer and a server without the gate do not change.

## Remote images

An image of a message that comes from the web shows only on a click, and never from your machine or your local network: an address such as `127.0.0.1`, `192.168.…` or `10.…`, `localhost`, or a name that leads there, even behind a proxy. The message says so: “An image of this machine or of the local network is not shown”.
