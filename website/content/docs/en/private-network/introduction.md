# Reach a private network

Agents that run on an Escouade server and must reach a git, a package registry or an API that can only be seen inside a company’s network: the four ways, how to choose, and what each party sees.

> [!WARNING]
> The private network is a preview of 2.3, like [server mode](server). The connector and its files are published with the 2.3 release.

## The need

Your Escouade server runs on a VPS (Coolify, Hetzner…), outside the network of the company you work for, Acme. Its services can only be seen inside its network, `*.acme.internal`, behind its VPN. Three cases come up:

| Case | The service | What the agent does |
|---|---|---|
| The git | `gitlab.acme.internal`, over SSH (port 22) | `git clone`, `git fetch`, `git push` from its worktree; Escouade also runs its own `git fetch` |
| The package registry | `nexus.acme.internal`, over HTTPS (443) | `npm install`, `mvn package`, `./gradlew build`, with the internal packages |
| The tests’ API | `api-recette.acme.internal:8443`, over HTTPS | The integration tests it runs call the staging API |

In every way, the credentials stay yours and live on the server: an SSH key added to the GitLab as a deploy key, the registry’s token in `.npmrc` or `settings.xml`, the staging token in the variables of the test launch, and Acme’s root certificate in Settings › Advanced › Network of the server’s space (“Extra certificate authorities”, see [the network](settings-network#certificates)).

What is not part of it: Acme’s network reaching the server (a webhook from an internal CI), UDP or ICMP (an internal DNS queried directly, a `ping`), a Windows file share, the remote desktop of an internal machine.

## The four ways

| | What it asks of the company | What a hijacked agent can reach | In Escouade |
|---|---|---|---|
| **V0, the server in the company’s network** | A machine in its network, its firewall rules, the way out to the models; accepting agents in its network | What Acme’s firewall allows that machine | Nothing to build: [the guide](private-network/company-server) |
| **V1, the server as a VPN client** | A VPN account for a machine, against its rule most of the time | Everything the VPN account reaches | Not built: [a recipe](private-network/vpn-client) |
| **V2, the connector** | Installing a program, writing its list, allowing one way out on 443. No inbound port | The connector’s list, at most | Built: [the connector](private-network/connector) |
| **V3, the computer as a relay** | Nothing, or accepting a use of its VPN it did not plan for | The relay’s list, kept by you | Not built |

- **V0**: `escouade-server` runs on one of Acme’s machines and reaches what its network allows; you connect to it over SSH through your VPN, as to any server. The bound is Acme’s firewall, with its tools and its logs.
- **V1**: the VPS itself joins Acme’s VPN, through a user-space VPN client that exposes a local proxy.
- **V2**: a small program, `escouade-connector`, runs on one of Acme’s machines. It opens an **outbound** connection to your server, and lets through only the agents’ connections to a list of hosts the IT department writes itself.
- **V3**: your laptop, on the VPN, would relay for the server.

## How to choose

1. **The company agrees to host the machine**: V0. It is the simplest, nothing new on its side but a machine, and the bound is its own. It is also the heaviest decision to ask of a CISO: agents that run commands inside its network.
2. **Otherwise, the server stays with you**: the connector (V2). The IT department installs a program, writes the list of what it lets through and keeps its hand on it; it opens no inbound port. Give it [the sheet for the IT department](private-network/it-sheet).
3. **The company gives you a VPN account for the server**, and nothing else: the V1 recipe, with its caveats.

## What V1 and V3 do not do

- **V1 is not built.** Escouade ships no VPN client: the recipe goes through a user-space client you install and look after yourself. A VPN account is usually a person’s, with strong authentication (MFA) that expires within hours: the server then loses the private network until your next gesture, and round-the-clock work with it. And nothing bounds what an agent reaches inside the VPN but what the company bounds on its concentrator.
- **V3 is not built, and will not be in this work.** It would only work with the laptop on, awake and on the VPN, which loses round-the-clock work, and would make Acme’s VPN serve a machine Acme does not manage, which a company VPN’s rule forbids most of the time. A dynamic reverse SSH forward (`ssh -R`, a SOCKS on the server that goes out through your computer) would have no list at all: everything your laptop reaches would become reachable by an agent. Do not do it.

## Where does my data go?

- **Escouade, the publisher, sees nothing of the private network**, in any way: no relay of ours, no telemetry from the connector. The tunnel runs from the connector to your server, without going through us.
- **Your server sees the names, ports, bytes and durations** of its agents’ connections, never the content of what is encrypted end to end between the agent and the internal host (SSH, TLS). A plain HTTP request passes through it without being read or kept.
- **The connector, at the company**, logs each connection the same way, without the content.
- **What the agents read in the private network** (the GitLab’s code, the API’s answers) stays on your server, in their worktrees and their conversations, like the rest of their work (see [what the server keeps](server#what-the-server-keeps)).

## Security, in one sentence

What an agent hijacked by a booby-trapped content can reach is what the last checkpoint in the company’s network lets through: its firewall in V0, the connector’s ceiling in V2, the VPN account in V1. A project’s list on the server is only one more defense, an instruction given to the agents, not a wall.
