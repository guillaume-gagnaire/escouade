# The host’s capabilities

A plugin in WASM reaches nothing by itself: no network, no disk, no program. This page says what it can ask Escouade for, what each capability allows, what Escouade refuses and what the install window says about it.

## The principle

A capability always goes the same way, for `run.js` and `run.wasm` alike:

- **Declared** in the manifest, in `permissions` or `limits`. What is not declared answers the error `-32003`, and the journal keeps it.
- **Enforced by Escouade**: the module has no socket, no file descriptor, no process. Each call goes through the host, which judges it.
- **Said** in the install window, line by line, before anything runs. The user can take each line away.
- **Kept in the journal**: the method and the host or the root, never the path, the query or the headers. A command goes there with its arguments, secrets hidden.

A plugin that asks for everything, to show the form:

`sdk/doc-examples/rust/escouade-plugin.json` (excerpt)

```json
  "permissions": {
    "events": ["ticket.column"],
    "network": ["api.example.dev", "{account}"],
    "secrets": ["token"],
    "files": [
      { "root": "plugin", "access": "read" },
      { "root": "temp", "access": "write" },
      { "root": "agent", "access": "write" },
      { "root": "agentHome", "access": "read" }
    ],
    "commands": ["git status --short", "git log *"]
  },
  "limits": { "fetchSeconds": 60, "fetchBytes": 8388608, "fileBytes": 1048576 },
```

| Capability | In the manifest | The install window says |
| --- | --- | --- |
| A host | `permissions.network` | “Connects to”, then the host |
| The instances of its accounts | `{account}` | “Connects to the instances you give it” |
| A harness’s local server | `local:shared` | “Talks to the harness’s local server, started by Escouade” |
| Reading or writing a folder | `permissions.files` | “Reads your agents’ files”, “Writes in your agents’ folders”… |
| Running a program | `permissions.commands` | “Runs this command on your computer”, then the command |
| Higher limits | `limits` | Nothing more |
| Timers | nothing, a basic right | Nothing |

## The network

Everything that goes out goes through `net/fetch`, `net/stream` or `ws/open`: see [the calls](plugins/calls#the-network). A plugin in JS keeps the standard `fetch`, a plugin in Rust the kit’s client.

### A named host

`"network": ["api.example.dev"]`.

**What it allows.** Reaching this host and its subdomains, over https. `*.example.dev` takes the subdomains without the host itself.

**What Escouade refuses.**

- http, and any other host;
- an IP address written in the URL, which `permissions.network` does not name;
- an address of the machine or of the local network, even for a declared host: loopback, private, link-local, unspecified, multicast or broadcast, in IPv4 as in IPv6. Escouade judges the addresses the connection uses, after the name is resolved;
- redirects: the plugin gets the 3xx response and decides. In Rust, the kit’s client follows them for it, and each hop is judged as a call.

A refused call answers `-32003`. A call that fails on the way answers `-32004`.

**Settings › Advanced › Network.** The proxy, its exclusions and unverified certificates are followed, and a change reaches the next call, without restarting the plugin. Through a proxy, only the name is judged, and the journal says so.

**The window** says “Connects to”, then each host, with “enforced by Escouade”.

### The instances of its accounts: `{account}`

`"network": ["{account}"]`, for a self-hosted service: GitHub Enterprise Server, GitLab, Gitea, Jira.

**What it allows.** Reaching the host the user typed for an account this plugin serves, on the port typed, and that host alone. For it, http and the local network are allowed: the user named it.

**What Escouade refuses.** Its subdomains and any other host. The list is read again at each call: an account removed closes its host.

**The window** says “Connects to the instances you give it”. `{account}` is not a host: the store takes it as it is.

On the bench, `escouade-plugin test <folder> --account <address>` stands for an account.

### A harness’s local server: `local:shared`

`"network": ["local:shared"]`, for a harness whose server Escouade starts, like `opencode serve`.

**What it allows.** http to `127.0.0.1:<port>`, only on a port the core read in the output of a shared process it started for this plugin, as long as that process lives. This traffic never goes through the proxy.

**What Escouade refuses.** Any other local port. The plugin never names a port the core did not read, and `local:shared` serves neither a WebSocket nor another plugin.

**The window** says “Talks to the harness’s local server, started by Escouade”.

### The limits: `limits`

`sdk/doc-examples/rust/escouade-plugin.json` (excerpt)

```json
  "limits": { "fetchSeconds": 60, "fetchBytes": 8388608, "fileBytes": 1048576 },
```

| Key | What it says | Without it | The host’s ceiling |
| --- | --- | --- | --- |
| `fetchSeconds` | How long a `net/fetch` has | 10 s | 120 s |
| `fetchBytes` | How large a response is | 1 MiB | 32 MiB |
| `fileBytes` | What an `fs/read` gives | 256 KiB | 4 MiB |

A value past the ceiling is brought down to it by the host (the manifest refuses it for `fileBytes`). The window says nothing more: a limit opens nothing.

### Streams and the WebSocket

They are not declared: they follow `network`.

- **`net/stream`** applies the rules of `net/fetch` at its opening, then a credit: the host stops reading the socket past 256 KiB not consumed, so a slow plugin slows the server and not the app’s memory. 8 streams at once. 120 s without a byte closes a stream, unless it is declared `"sse": true`.
- **`ws/open`** is `wss` only, to the same hosts, never `local:shared`. 4 sockets, 1 MiB a message. `ws/send` is refused when what waits would pass 256 KiB. Its headers do not reach the journal.

## The files: `files`

`sdk/doc-examples/rust/escouade-plugin.json` (excerpt)

```json
    "files": [
      { "root": "plugin", "access": "read" },
      { "root": "temp", "access": "write" },
      { "root": "agent", "access": "write" },
      { "root": "agentHome", "access": "read" }
    ],
```

**What it allows.** `fs/read`, `fs/write`, `fs/stat`, `fs/list` and `fs/canon`, under a root of this closed list, which Escouade resolves:

| Root | What it is | The window says |
| --- | --- | --- |
| `plugin` | Its folder, read only: what the store signed does not change. | “Reads its own folder” |
| `temp` | A folder of its own, shared by its agents. | “Keeps files in a folder of its own” |
| `agent` | The agent’s working folder. | “Reads your agents’ files”, or “Writes in your agents’ folders” in orange |
| `agentRepo` | Its git repository, for a worktree. | The same |
| `agentHome` | Its account’s folder, where a harness keeps its credentials. | “Reads your agents’ account folder” |

`access` is `read` or `write`. A plugin names a root and a relative path, or a path of the host under a root of this agent: that is what a harness reads. Only harness plugins get an agent, through `agent/attach`, with the roots they declare and no others; another plugin has `plugin` and `temp`.

**What Escouade refuses.**

- a free path, a `~`, a `..`, an NTFS stream (`a.txt:b`), a link that leads nowhere;
- what leaves a root once links are followed. The refusal comes before saying whether the file exists: it tells nothing;
- `plugin` for writing, when the manifest is read;
- the folder of a private project the agent is cut off from, whatever the plugin’s code;
- a read of more than 256 KiB, or of `limits.fileBytes`.

Each write and each refusal go to the journal.

**The review** asks what `agentHome` is for: this folder holds a harness’s credentials.

## The programs: `commands`

`sdk/doc-examples/rust/escouade-plugin.json` (excerpt)

```json
    "commands": ["git status --short", "git log *"]
```

**What it allows.** `proc/spawn` runs one of these lines. The first word is a program’s name, which Escouade looks for on the PATH.

**What Escouade refuses.**

- a path in place of the name;
- what is not an entry, word for word; more words are allowed only when the entry ends in `*`;
- a shell: under Windows, neither `.cmd` nor `.bat`, so no tool installed by npm;
- a `dir` other than a root (`agent`, `project`), and a private project where the plugin is not on. Without `dir`, the program runs in `temp`;
- an `ESCOUADE_*`, `PATH`, `LD_*`, `DYLD_*` or `NODE_OPTIONS` variable in `env`: it would run something else.

Each program dies with the plugin’s instance or at `proc/kill`, with what it started. Four at once. Its output is no longer read past 256 KiB that have not reached the module.

**The window** says “Runs this command on your computer”, then the command, in red, one line a command.

> [!WARNING]
> The module is isolated, the program it runs is not: it has the user’s rights. An entry in `*` on a general program (`git *`, `docker *`) opens everything again, through `git -c core.sshCommand=…` for example. This is the strongest capability: ask for exact lines, and expect to justify a `*` at the review.

`permissions.commands` is not `contributes.commands`, which runs a shell line on a fact, after the user’s agreement ([the events](plugins/events#a-command-on-a-fact)).

## The timers

`timer/set`, `timer/cancel` and `timer/fired` are not declared: they are a basic right, like `log`. The floor is 10 ms, what is below is brought up to it, and 64 timers at once. They stop with the instance. `setTimeout` in JS and the kit’s `sleep` in Rust go through them.

## Next

- [The calls](plugins/calls): each method, with an example in TS and in Rust.
- [The permissions](plugins/permissions): everything a plugin can ask for, as the user sees it.
- [Plugin security](plugins/security): what the review does with it.
