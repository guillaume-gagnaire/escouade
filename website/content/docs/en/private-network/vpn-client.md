# The server as a VPN client

A recipe to bring your Escouade server into the company’s VPN through a user-space client, without rights on the machine, behind the agents’ gate; and its caveats, which make it rarely possible.

> [!WARNING]
> This way (V1) is not built into Escouade: it is a recipe, with tools you install and look after yourself. Escouade ships no VPN client. Prefer [the connector](private-network/connector) or [the server in the company’s network](private-network/company-server).

## First, the caveats

- **A VPN account for a machine.** A company VPN gives an account to a person, with strong authentication (MFA, SAML) and often a check of the device. An account for a server outside the company is the exception: ask the IT department for it, never use your own.
- **Strong authentication breaks round-the-clock work.** With MFA, the session expires (often after 8 to 12 hours) and asks for a gesture: the server loses the private network until then, nights included.
- **Whatever the VPN account reaches, an agent reaches.** Nothing, on Escouade’s side, bounds the company’s network in this way: the project lists of the [private network](private-network/connector#choose-a-project-s-list) only hold for connectors. The only bound is the one the company puts on that account, in its VPN concentrator; ask it to limit it to the hosts your projects need.

## The principle

A classic VPN client wants a network interface and administrator rights; the server’s image has none. User-space clients carry the VPN through a local proxy, without asking anything of the system:

```
agents ──▶ the gate (127.0.0.1) ──▶ the space’s proxy: Privoxy (127.0.0.1:8118)
                                        │ *.acme.internal              │ the rest
                                        ▼                              ▼
                       the VPN client’s SOCKS (127.0.0.1:1080)     the Internet, directly
                                        ▼
                                  Acme’s VPN
```

1. The VPN client exposes a **SOCKS** proxy on `127.0.0.1`: `wireproxy` for WireGuard, `openconnect` with `ocproxy` for AnyConnect or GlobalProtect. FortiClient and Zscaler Private Access want the system’s rights: no recipe for them.
2. **Privoxy** in front of it, a local HTTP proxy, sends the company’s names to the SOCKS proxy and the rest directly. Escouade’s gate talks only to an `http://` proxy.
3. Privoxy becomes the server’s **space proxy**, and the gate gives it to every agent, `ssh` included.

## The VPN client

### WireGuard, with wireproxy

The WireGuard file the IT department gives, and `wireproxy`’s configuration (on your server, readable by its account alone):

```ini
WGConfig = /home/escouade/vpn/acme.conf

[Socks5]
BindAddress = 127.0.0.1:1080
```

`wireproxy -c /home/escouade/vpn/wireproxy.conf`; names resolve through the DNS the WireGuard file gives (`DNS = …`).

### AnyConnect or GlobalProtect, with ocproxy

```sh
openconnect --script-tun --script "ocproxy -D 1080" vpn.acme.example
openconnect --protocol=gp --script-tun --script "ocproxy -D 1080" vpn.acme.example
```

`ocproxy` resolves names through the VPN’s DNS. The connection asks for your gesture at each strong authentication.

## Privoxy, the split

In its configuration (`/etc/privoxy/config` on a VPS):

```
listen-address 127.0.0.1:8118
forward-socks5t .acme.internal 127.0.0.1:1080 .
```

And, for `git` over SSH to go through (Privoxy otherwise bounds the ports of `CONNECT`), in `user.action`:

```
{ +limit-connect{,} }
.acme.internal
```

The rest of the Internet goes out directly, without going through the VPN.

## In Escouade

1. **The gate**: start the server with `--gate` (`ESCOUADE_SERVER_GATE=1`). It becomes the agents’ one proxy, and their `ssh` goes through it (`escouade-server via`); without it, the git over SSH would not go through the proxy.
2. **The space’s proxy**: connected to the server’s space, Settings › Advanced › Network, “HTTP(S) proxy” set to `http://127.0.0.1:8118`, “Exclusions” set to `localhost,127.0.0.1`. The gate takes it behind itself, for everything that does not go to a connector (see [the network](settings-network)).
3. **The company’s root certificate** in “Extra certificate authorities”, for the registry and the staging API.

`git clone git@gitlab.acme.internal:acme/api.git` then goes through the gate, Privoxy, the SOCKS proxy and the VPN; so does `npm install` for the registry.

## In Docker

The server runs without any capability, and must. Put the VPN client and Privoxy in a container beside it that shares its network (`network_mode: "service:escouade"` in the Compose file): their `127.0.0.1` is the server’s, and the server’s image does not change. The client’s container needs no rights either: that is the whole point of a user-space client.

## Undo it

Empty the space’s “HTTP(S) proxy”, stop Privoxy and the VPN client. And tell the IT department the server’s VPN account can be closed.
