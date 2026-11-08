# Settings: the network (Advanced)

The “Network” group of the “Advanced” page configures the HTTP(S) proxy and TLS certificate checking for Claude Code, quotas, integrations, updates, and dictation (the download of its models and, online, its service).

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

The app says so under the group’s title: “Integrations, quotas, and updates trust recognized certificates and those installed on this system (usually a corporate proxy’s). Claude Code has its own list.”

| Setting | Effect | Default |
|---|---|---|
| “Skip TLS certificate verification” | Escouade no longer verifies TLS certificates. For a proxy that decrypts traffic with an unrecognized certificate (the “UnknownIssuer” error). | Off |

When on, the setting applies to integrations, quotas, and updates, and to Claude Code processes (the next time their process starts): they get `NODE_TLS_REJECT_UNAUTHORIZED=0`, which the commands agents run (npm, node…) inherit too. It doesn’t apply to the built-in terminals or to launch commands.

> [!WARNING]
> Use it only on a network you trust: an intercepted connection, tokens included, would no longer be detected. If your proxy has its own certificate, install it on the system rather than turning verification off: integrations, quotas, and updates will recognize it (Claude Code has its own list).
