# Settings: the network (Advanced)

The “Network” group of the “Advanced” page configures the HTTP(S) proxy and TLS certificates (an extra authorities file, the checking) for Claude Code, quotas, integrations, updates, and dictation (the download of its models and, online, its service).

## Open the page

Open the settings with `Ctrl+,` (`⌘,` on macOS), then choose “Advanced”: “Network” is its first group; “proxy” in the bar’s search leads there too. These are the app’s settings, shared by all projects, and they apply as you set them.

They matter behind a company network: a proxy to get out, and sometimes a certificate the proxy imposes. Without a proxy, there’s nothing to set here.

## Proxy

The app says so under the group’s title: “Claude Code processes, quota reads, integrations, and updates all go through the proxy. It applies to agents the next time their process (re)starts.”

| Setting | Effect | Default |
|---|---|---|
| “HTTP(S) proxy” | The proxy’s address, for example `http://user:password@proxy:3128`. Empty (“none”), connections are direct. | Empty |
| “Exclusions” | The hosts that don’t go through the proxy, separated by commas (the `NO_PROXY` list). | `localhost,127.0.0.1` |
| “Also apply the proxy to the built-in terminals” | Terminals also get the proxy variables. | Off |

### What goes through the proxy

- **Claude Code processes**: agents, and the other calls to Claude Code that Escouade makes (naming an agent, commit messages, signing in to an account). They get `HTTPS_PROXY`, `HTTP_PROXY`, their lowercase variants, and `NO_PROXY` (with `no_proxy`) if “Exclusions” isn’t empty. An agent only sees them the next time its process (re)starts: an agent already running keeps its settings until then.
- **Quota reads and integrations** (Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues, Azure Boards): the app’s requests go through the proxy and respect “Exclusions”.
- **Updates**: checking and downloading go through the proxy. An invalid proxy address makes the check fail, with the message “invalid proxy address”. When the proxy itself doesn’t answer (switched off, or a computer outside the company network), Escouade tries once without the proxy and the failure message gives both tries; when the proxy refuses the request, it isn’t made again without the proxy. See [“Behind a company proxy”](updates#behind-a-company-proxy).
- **Built-in terminals**: only if “Also apply the proxy to the built-in terminals” is on. The same variables are then added to terminals, launch commands, and worktree commands. See [terminals](terminals).

Without a proxy address, Escouade adds no proxy variable: processes keep your session’s environment.

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
