# Escouade in your company’s network

Run the Escouade server on one of the company’s machines, which reaches what its network allows, and connect to it over SSH through your VPN: the machine, the outbound proxy, the root certificate, what the IT department does.

## When to take it

It is way V0 of [Reach a private network](private-network/introduction): nothing new to install at the company but a machine, and the bound is its firewall, with its tools, its logs and its data loss prevention. Its outbound proxy sees what the agents send to the Internet.

It asks the company to host agents that run commands inside its network. If it refuses, take [the connector](private-network/connector).

## Who does what

| You | The IT department |
|---|---|
| Ask for a Linux machine, x64 or arm64 (a VM) | Provide it, on a segment of its network |
| Install `escouade-server` on it: the [systemd service](server#without-docker-with-systemd) or [the image](server#with-docker) | Open, from that segment, the GitLab, the registry and the staging API |
| Connect to it over SSH through your VPN | Open the way out to the Internet for the models’ APIs, through its proxy |
| Set the proxy and the root certificate in the server’s space | Give its proxy’s address and its root certificate (the one of a TLS inspection) |
| Put the credentials of the three cases on the server (SSH key, registry token, staging token) | Accept that agents run inside its network |

## Install the server

As on a VPS: see [server mode](server). Two things change:

- **The download goes through the company’s proxy.** `install.sh` and `docker pull` follow `HTTPS_PROXY`: run them with the variable set (`HTTPS_PROXY=http://proxy.acme.internal:3128 sh install.sh`), or set Docker’s proxy on the machine. If the proxy inspects TLS, its root certificate must be in the machine’s store (on Debian, a `.crt` file in `/usr/local/share/ca-certificates`, then `update-ca-certificates`).
- **The server’s automatic update** reads and downloads its versions the same way: the proxy must let `github.com` and its downloads through.

## Connect through the VPN

Your computer, on the VPN, reaches the machine over SSH as any server: see [Connect to a server](connect-server). A host in `~/.ssh/config`, with a bastion if needed:

```
Host acme-escouade
  HostName escouade.acme.internal
  User escouade
  ProxyJump bastion.acme.internal
```

Escouade runs your computer’s `ssh`, with your config, your agent and your known hosts: the `ProxyJump` follows. With the VPN down, the connection drops, and Escouade restarts it on its own (see [when the connection drops](connect-server#when-the-connection-drops)); the agents go on working on the server.

Your phone reaches this server only through the company’s VPN.

## The outbound proxy and the root certificate

The harnesses reach their models on the Internet: through the company’s proxy, if it is the only way out. Connected to the server’s space, open Settings › Advanced › Network: these settings are the server’s, and go to its agents (see [the network](settings-network)).

| Setting | What to put there |
|---|---|
| “HTTP(S) proxy” | `http://proxy.acme.internal:3128`, with a user name and password if the proxy asks for them (see [the proxy’s password](settings-network#the-proxy-s-password)). |
| “Exclusions” | What does not go through the proxy: `localhost,127.0.0.1,.acme.internal`, so that the GitLab, the registry and the API are reached directly |
| “Never go out without the proxy” | On: nothing leaves directly if the proxy does not answer |
| “Extra certificate authorities” | The company’s root certificate, as `.pem`: the Node harnesses get it as `NODE_EXTRA_CA_CERTS`, Codex as `CODEX_CA_CERTIFICATE`, and the agents’ commands inherit it |
| “Also apply the proxy to the built-in terminals” | On if you run `npm` or `git` by hand in the server’s terminals |

The agents get these settings the next time their process starts.

> [!NOTE]
> Escouade’s authorities file goes to the harnesses and to what reads `NODE_EXTRA_CA_CERTS`, not to everything. `git` over HTTPS, Python and Java read the machine’s store: put the root certificate there too, or, in Docker, mount it in the image and set `GIT_SSL_CAINFO`, `REQUESTS_CA_BUNDLE` or Java’s store.

## The three cases

- **The git over SSH**: the deploy key on the server (in `~/.ssh`, in Docker `/data/.ssh`), the GitLab’s host key in `known_hosts`, checked once by hand (`ssh -T git@gitlab.acme.internal`). The GitLab is reached directly: no proxy for SSH.
- **The registry**: the token in `.npmrc` (`//nexus.acme.internal/repository/npm/:_authToken=…`) or `settings.xml`. Maven and Gradle do not follow `HTTPS_PROXY`: for Maven Central through the proxy, a `<proxies>` section in `settings.xml`, or `systemProp.https.proxyHost` in `gradle.properties`, which a project’s setup step can write.
- **The tests’ API**: its token in the variables of the test launch, never in the repository.

## What an agent can reach

What the firewall allows that machine, and the Internet through the company’s proxy. A hijacked agent that would want to take data out goes through the proxy that already watches it. Ask the IT department to bound the machine’s segment to the three hosts your projects need, rather than the whole network.
